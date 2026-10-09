use crate::database::repositories::{
    meeting::MeetingsRepository, setting::SettingsRepository, summary::SummaryProcessesRepository,
};
use crate::summary::llm_client::LLMProvider;
use crate::summary::processor::{
    clean_llm_markdown_detailed, extract_meeting_name_from_markdown, generate_meeting_summary,
    require_visible_markdown,
};
use crate::summary::templates;
use crate::ollama::metadata::ModelMetadataCache;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

static METADATA_CACHE: LazyLock<ModelMetadataCache> =
    LazyLock::new(|| ModelMetadataCache::new(Duration::from_secs(300)));

#[derive(Clone)]
struct RegisteredCancellation {
    started_at: DateTime<Utc>,
    token: CancellationToken,
}

static CANCELLATION_REGISTRY: LazyLock<Arc<Mutex<HashMap<String, RegisteredCancellation>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Strips the first `#` heading line; returns "" if no `#` is found.
fn strip_leading_title(markdown: &str) -> String {
    if let Some(hash_pos) = markdown.find('#') {
        let body_start = markdown[hash_pos..]
            .find('\n')
            .map_or(markdown.len(), |line_end| hash_pos + line_end);
        markdown[body_start..].trim_start().to_string()
    } else {
        String::new()
    }
}

/// Strips the leading H1 (`# Title\n...`) only when the markdown starts with one.
/// No-op on already-stripped values, values starting with `## Subheading`, or values
/// without any heading. Avoids the silent-empty-return case where `strip_leading_title`
/// returns "" for input lacking a leading `#`.
fn strip_title_if_present(markdown: &str) -> String {
    if markdown.trim_start().starts_with("# ") {
        strip_leading_title(markdown)
    } else {
        markdown.to_string()
    }
}

/// Builds the `summary_processes.result` JSON: display Markdown without its
/// leading title, plus whether model reasoning was stripped.
fn build_summary_result_json(
    final_markdown: &str,
    reasoning_stripped: bool,
) -> Result<serde_json::Value, String> {
    let cleaned_final = clean_llm_markdown_detailed(final_markdown);
    require_visible_markdown("Final summary", &cleaned_final)?;
    let markdown = strip_title_if_present(&cleaned_final.markdown);
    if markdown.trim().is_empty() {
        return Err("Final summary contains no visible content after title removal".to_string());
    }

    Ok(serde_json::json!({
        "markdown": markdown,
        "reasoning_stripped": reasoning_stripped || cleaned_final.reasoning_stripped,
    }))
}

/// Summary service - handles all summary generation logic
pub struct SummaryService;

impl SummaryService {
    /// Registers a new cancellation token for a meeting.
    pub(crate) fn register_cancellation_token(
        meeting_id: &str,
        started_at: DateTime<Utc>,
    ) -> CancellationToken {
        let token = CancellationToken::new();
        let mut registry = CANCELLATION_REGISTRY
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(previous) = registry.insert(
            meeting_id.to_string(),
            RegisteredCancellation {
                started_at,
                token: token.clone(),
            },
        ) {
            previous.token.cancel();
        }
        info!("Registered cancellation token for meeting: {}", meeting_id);
        token
    }

    /// Cancels only the active generation identified by `started_at`.
    pub fn cancel_summary(meeting_id: &str, started_at: DateTime<Utc>) -> bool {
        let registry = CANCELLATION_REGISTRY
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(entry) = registry.get(meeting_id).filter(|entry| entry.started_at == started_at)
        {
            info!("Cancelling summary generation for meeting: {}", meeting_id);
            entry.token.cancel();
            true
        } else {
            warn!("No active summary generation found for meeting: {}", meeting_id);
            false
        }
    }

    /// Cleans up only the matching generation token after processing completes.
    fn cleanup_cancellation_token(meeting_id: &str, started_at: DateTime<Utc>) {
        let mut registry = CANCELLATION_REGISTRY
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if registry
            .get(meeting_id)
            .is_some_and(|entry| entry.started_at == started_at)
        {
            registry.remove(meeting_id);
            info!("Cleaned up cancellation token for meeting: {}", meeting_id);
        }
    }

    /// Processes transcript in the background and generates summary
    ///
    /// This function is designed to be spawned as an async task and does not block
    /// the main thread. It updates the database with progress and results.
    ///
    /// # Arguments
    /// * `_app` - Tauri app handle (for future use)
    /// * `pool` - SQLx connection pool
    /// * `meeting_id` - Unique identifier for the meeting
    /// * `text` - Full transcript text
    /// * `model_provider` - LLM provider name (e.g., "ollama", "openai")
    /// * `model_name` - Specific model (e.g., "gpt-4", "llama3.2:latest")
    /// * `custom_prompt` - Optional user-provided context
    /// * `template_id` - Template identifier (e.g., "daily_standup", "standard_meeting")
    pub async fn process_transcript_background<R: tauri::Runtime>(
        _app: AppHandle<R>,
        pool: SqlitePool,
        meeting_id: String,
        started_at: DateTime<Utc>,
        cancellation_token: CancellationToken,
        text: String,
        model_provider: String,
        model_name: String,
        custom_prompt: String,
        template_id: String,
    ) {
        let start_time = Instant::now();
        info!(
            "Starting background processing for meeting_id: {}",
            meeting_id
        );

        // Parse provider
        let provider = match LLMProvider::from_str(&model_provider) {
            Ok(p) => p,
            Err(e) => {
                Self::fail_and_cleanup(&pool, &meeting_id, started_at, &e).await;
                return;
            }
        };

        // Validate and setup api_key, Flexible for Ollama, BuiltInAI, and CustomOpenAI
        let api_key = if provider == LLMProvider::Ollama || provider == LLMProvider::BuiltInAI || provider == LLMProvider::CustomOpenAI {
            // These providers don't require API keys from the standard database column
            String::new()
        } else {
            match SettingsRepository::get_api_key(&pool, &model_provider).await {
                Ok(Some(key)) if !key.is_empty() => key,
                Ok(None) | Ok(Some(_)) => {
                    let err_msg = format!("API key not found for {}", &model_provider);
                    Self::fail_and_cleanup(&pool, &meeting_id, started_at, &err_msg).await;
                    return;
                }
                Err(e) => {
                    let err_msg = format!("Failed to retrieve API key for {}: {}", &model_provider, e);
                    Self::fail_and_cleanup(&pool, &meeting_id, started_at, &err_msg).await;
                    return;
                }
            }
        };

        // Get Ollama endpoint if provider is Ollama
        let ollama_endpoint = if provider == LLMProvider::Ollama {
            match SettingsRepository::get_model_config(&pool).await {
                Ok(Some(config)) => config.ollama_endpoint,
                Ok(None) => None,
                Err(e) => {
                    info!("Failed to retrieve Ollama endpoint: {}, using default", e);
                    None
                }
            }
        } else {
            None
        };

        // Get CustomOpenAI config if provider is CustomOpenAI
        let (custom_openai_endpoint, custom_openai_api_key, custom_openai_max_tokens, custom_openai_temperature, custom_openai_top_p) =
            if provider == LLMProvider::CustomOpenAI {
                match SettingsRepository::get_custom_openai_config(&pool).await {
                    Ok(Some(config)) => {
                        info!("✓ Using custom OpenAI endpoint: {}", config.endpoint);
                        (
                            Some(config.endpoint),
                            config.api_key,
                            config.max_tokens.map(|t| t as u32),
                            config.temperature,
                            config.top_p,
                        )
                    }
                    Ok(None) => {
                        let err_msg = "Custom OpenAI provider selected but no configuration found";
                        Self::fail_and_cleanup(&pool, &meeting_id, started_at, err_msg).await;
                        return;
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to retrieve custom OpenAI config: {}", e);
                        Self::fail_and_cleanup(&pool, &meeting_id, started_at, &err_msg).await;
                        return;
                    }
                }
            } else {
                (None, None, None, None, None)
            };

        // For CustomOpenAI, use its API key (if any) instead of the empty string
        let final_api_key = if provider == LLMProvider::CustomOpenAI {
            custom_openai_api_key.unwrap_or_default()
        } else {
            api_key
        };

        // Dynamically fetch context size based on provider and model
        let token_threshold = if provider == LLMProvider::Ollama {
            match METADATA_CACHE.get_or_fetch(&model_name, ollama_endpoint.as_deref()).await {
                Ok(metadata) => {
                    // Reserve 300 tokens for prompt overhead
                    let optimal = metadata.context_size.saturating_sub(300);
                    info!(
                        "✓ Using dynamic context for {}: {} tokens (chunk size: {})",
                        model_name, metadata.context_size, optimal
                    );
                    optimal
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch context for {}: {}. Using default 4000",
                        model_name, e
                    );
                    4000  // Fallback to safe default
                }
            }
        } else if provider == LLMProvider::BuiltInAI {
            // Get model's context size from registry
            use crate::summary::summary_engine::models;
            let model = models::get_model_by_name(&model_name)
                .ok_or_else(|| format!("Unknown model: {}", model_name));

            match model {
                Ok(model_def) => {
                    // Reserve 300 tokens for prompt overhead
                    let optimal = model_def.context_size.saturating_sub(300) as usize;
                    info!(
                        "✓ Using BuiltInAI context size: {} tokens (chunk size: {})",
                        model_def.context_size, optimal
                    );
                    optimal
                }
                Err(e) => {
                    warn!("{}, using default 2048", e);
                    1748  // 2048 - 300 for overhead
                }
            }
        } else {
            // Cloud providers (OpenAI, Claude, Groq, CustomOpenAI) handle large contexts automatically
            100000  // Effectively unlimited for single-pass processing
        };

        // Get app data directory for BuiltInAI provider
        let app_data_dir = _app.path().app_data_dir().ok();

        let template = match templates::get_template(&template_id) {
            Ok(template) => template,
            Err(e) => {
                let err_msg = format!("Failed to load template '{}': {}", template_id, e);
                Self::fail_and_cleanup(&pool, &meeting_id, started_at, &err_msg).await;
                return;
            }
        };
        let client = reqwest::Client::new();
        let result = generate_meeting_summary(
            &client,
            &provider,
            &model_name,
            &final_api_key,
            &text,
            &custom_prompt,
            &template_id,
            &template,
            token_threshold,
            ollama_endpoint.as_deref(),
            custom_openai_endpoint.as_deref(),
            custom_openai_max_tokens,
            custom_openai_temperature,
            custom_openai_top_p,
            app_data_dir.as_ref(),
            Some(&cancellation_token),
        )
        .await;

        let duration = start_time.elapsed().as_secs_f64();

        match result {
            Ok(generated) => {
                info!(
                    "✓ Successfully processed {} chunks for meeting_id: {}. Duration: {:.2}s",
                    generated.successful_chunk_count, meeting_id, duration
                );
                let result_json = match build_summary_result_json(
                    &generated.final_markdown,
                    generated.reasoning_stripped,
                ) {
                    Ok(result) => result,
                    Err(error) => {
                        Self::update_process_failed(&pool, &meeting_id, started_at, &error).await;
                        Self::cleanup_cancellation_token(&meeting_id, started_at);
                        return;
                    }
                };

                match SummaryProcessesRepository::update_process_completed(
                    &pool,
                    &meeting_id,
                    started_at,
                    result_json,
                    generated.successful_chunk_count,
                    duration,
                )
                .await
                {
                    Ok(true) => {
                        if let Some(name) =
                            extract_meeting_name_from_markdown(&generated.final_markdown)
                                .filter(|name| !name.is_empty())
                        {
                            if let Err(error) =
                                MeetingsRepository::update_meeting_name(&pool, &meeting_id, &name)
                                    .await
                            {
                                error!("Failed to update meeting name for {}: {}", meeting_id, error);
                            }
                        }
                        info!("Summary saved successfully for meeting_id: {}", meeting_id);
                    }
                    Ok(false) => warn!("Skipped stale summary completion for meeting_id: {}", meeting_id),
                    Err(error) => error!(
                        "Failed to save completed process for {}: {}",
                        meeting_id, error
                    ),
                }
            }
            Err(error) if cancellation_token.is_cancelled() => {
                match SummaryProcessesRepository::update_process_cancelled(
                    &pool,
                    &meeting_id,
                    started_at,
                )
                .await
                {
                    Ok(false) => warn!("Skipped stale summary cancellation for meeting_id: {}", meeting_id),
                    Ok(true) => info!("Summary generation was cancelled for meeting_id: {}", meeting_id),
                    Err(db_error) => error!(
                        "Failed to update DB status to cancelled for {}: {}",
                        meeting_id, db_error
                    ),
                }
            }
            Err(error) => {
                Self::update_process_failed(&pool, &meeting_id, started_at, &error).await;
            }
        }
        Self::cleanup_cancellation_token(&meeting_id, started_at);
    }

    /// Updates the summary process status to failed with error message
    async fn fail_and_cleanup(
        pool: &SqlitePool,
        meeting_id: &str,
        started_at: DateTime<Utc>,
        error_msg: &str,
    ) {
        Self::update_process_failed(pool, meeting_id, started_at, error_msg).await;
        Self::cleanup_cancellation_token(meeting_id, started_at);
    }

    async fn update_process_failed(
        pool: &SqlitePool,
        meeting_id: &str,
        started_at: DateTime<Utc>,
        error_msg: &str,
    ) {
        error!(
            "Processing failed for meeting_id {}: {}",
            meeting_id, error_msg
        );
        match SummaryProcessesRepository::update_process_failed(
            pool,
            meeting_id,
            started_at,
            error_msg,
        )
        .await
        {
            Ok(false) => warn!("Skipped stale summary failure for meeting_id: {}", meeting_id),
            Ok(true) => {}
            Err(e) => error!(
                "Failed to update DB status to failed for {}: {}",
                meeting_id, e
            ),
        }
    }


}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_cleanup_keeps_the_replacement_cancellation_token() {
        let meeting_id = format!("summary-test-{}", uuid::Uuid::new_v4());
        let first_started_at = Utc::now();
        let second_started_at = first_started_at + chrono::Duration::nanoseconds(1);
        let first = SummaryService::register_cancellation_token(&meeting_id, first_started_at);
        let second = SummaryService::register_cancellation_token(&meeting_id, second_started_at);

        assert!(first.is_cancelled());
        SummaryService::cleanup_cancellation_token(&meeting_id, first_started_at);
        assert!(SummaryService::cancel_summary(&meeting_id, second_started_at));
        assert!(second.is_cancelled());
        SummaryService::cleanup_cancellation_token(&meeting_id, second_started_at);
    }

    #[test]
    fn test_strip_leading_title_with_body() {
        let input = "# Meeting Title\nThis is the body.\nMore content.";
        let result = strip_leading_title(input);
        assert_eq!(result, "This is the body.\nMore content.");
    }

    #[test]
    fn test_strip_leading_title_only() {
        let input = "# Meeting Title";
        let result = strip_leading_title(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_strip_leading_title_no_heading() {
        let input = "No heading here.\nJust body.";
        let result = strip_leading_title(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_strip_leading_title_multiline_body() {
        let input = "# Title\n## Subheading\nParagraph 1\n\nParagraph 2";
        let result = strip_leading_title(input);
        assert_eq!(result, "## Subheading\nParagraph 1\n\nParagraph 2");
    }

    #[test]
    fn test_strip_leading_title_empty_after_heading() {
        let input = "# Title\n";
        let result = strip_leading_title(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_strip_leading_title_whitespace_after_heading() {
        let input = "# Title\n   \n Body with leading spaces";
        let result = strip_leading_title(input);
        assert_eq!(result, "Body with leading spaces");
    }

    #[test]
    fn test_strip_title_if_present_preserves_already_stripped() {
        assert_eq!(strip_title_if_present("## Action Items\nfoo"), "## Action Items\nfoo");
    }

    #[test]
    fn test_strip_title_if_present_strips_leading_h1() {
        assert_eq!(strip_title_if_present("# Meeting Title\n## Action Items\nfoo"), "## Action Items\nfoo");
    }

    #[test]
    fn test_strip_title_if_present_no_heading_preserved() {
        // Distinct from strip_leading_title which returns "" — this preserves input.
        assert_eq!(strip_title_if_present("Just body text"), "Just body text");
    }

    #[test]
    fn test_strip_title_if_present_hash_no_space_preserved() {
        // `#NoSpace` is not a markdown H1 — preserve.
        assert_eq!(strip_title_if_present("#NoSpace\nbody"), "#NoSpace\nbody");
    }

    #[test]
    fn test_strip_title_if_present_mid_document_h1_preserved() {
        // H1 after body content must NOT be stripped — guards the asymmetry where
        // extract_meeting_name_from_markdown scans every line for "# ".
        let input = "Some paragraph\n\n# H1 on line 3\n## Section\nbody";
        assert_eq!(strip_title_if_present(input), input);
    }

    #[test]
    fn test_strip_title_if_present_leading_whitespace_h1_stripped() {
        assert_eq!(
            strip_title_if_present("  # Title\n## Section\nbody"),
            "## Section\nbody"
        );
    }

    #[test]
    fn result_json_rejects_title_only_display_markdown() {
        assert_eq!(
            build_summary_result_json("# Title", false),
            Err("Final summary contains no visible content after title removal".to_string())
        );
    }

    #[test]
    fn result_json_never_persists_reasoning_text() {
        let result = build_summary_result_json("# Title\nVisible", true).unwrap();
        assert_eq!(result["reasoning_stripped"], true);
        assert!(result.get("reasoning").is_none());
    }
}
