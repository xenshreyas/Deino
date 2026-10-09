//! Background summary jobs: the `summary_processes` lifecycle around the
//! gateway-backed `deino_core::summaries::SummaryGenerator` (FND-09).
//!
//! A job resolves the provider and model the UI chose, loads the template,
//! asks the generator, and records completion, cancellation or failure on
//! its `summary_processes` row. Only the generation identified by its start
//! time can complete or cancel a row, so a stale job never overwrites a
//! newer one. FND-08 turns this into a pipeline stage; NOT-02 replaces it.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use chrono::{DateTime, Utc};
use deino_core::summaries::{GeneratedSummary, SummaryGenerator, SummaryRequest, first_heading};
use deino_llm::{ProviderId, Target};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager, Runtime};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use super::result::build_summary_result_json;
use crate::database::repositories::{
    meeting::MeetingsRepository, summary::SummaryProcessesRepository,
};
use crate::summary::templates;

/// The cancellation token of each meeting's running summary, with the start
/// time that identifies the generation.
static CANCELLATION_REGISTRY: LazyLock<Mutex<HashMap<String, (DateTime<Utc>, CancellationToken)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// One background summary.
pub(crate) struct SummaryJob {
    /// The writer pool, for `summary_processes` and the meeting name.
    pub pool: SqlitePool,
    /// The meeting being summarised.
    pub meeting_id: String,
    /// Identifies this generation of the meeting's summary.
    pub started_at: DateTime<Utc>,
    /// Cancelled by `api_cancel_summary` or a newer generation.
    pub cancellation: CancellationToken,
    /// The transcript.
    pub text: String,
    /// The provider as the UI names it ("claude", "builtin-ai", ...).
    pub provider: String,
    /// The provider's model.
    pub model: String,
    /// Context the user typed, possibly empty.
    pub custom_prompt: String,
    /// The summary template's ID.
    pub template_id: String,
}

/// How a job ended, short of success.
enum Stopped {
    Cancelled,
    Failed(String),
}

/// Summary jobs and their cancellation.
pub struct SummaryService;

impl SummaryService {
    /// Registers a new cancellation token for a meeting, cancelling the
    /// previous generation's.
    pub(crate) fn register_cancellation_token(
        meeting_id: &str,
        started_at: DateTime<Utc>,
    ) -> CancellationToken {
        let token = CancellationToken::new();
        let previous = registry().insert(meeting_id.to_string(), (started_at, token.clone()));
        if let Some((_, previous)) = previous {
            previous.cancel();
        }
        info!("Registered cancellation token for meeting: {}", meeting_id);
        token
    }

    /// Cancels only the active generation identified by `started_at`.
    pub fn cancel_summary(meeting_id: &str, started_at: DateTime<Utc>) -> bool {
        let registry = registry();
        match registry.get(meeting_id).filter(|(at, _)| *at == started_at) {
            Some((_, token)) => {
                info!("Cancelling summary generation for meeting: {}", meeting_id);
                token.cancel();
                true
            }
            None => {
                warn!(
                    "No active summary generation found for meeting: {}",
                    meeting_id
                );
                false
            }
        }
    }

    /// Removes the matching generation's token once it is done.
    fn cleanup_cancellation_token(meeting_id: &str, started_at: DateTime<Utc>) {
        let mut registry = registry();
        if registry
            .get(meeting_id)
            .is_some_and(|(at, _)| *at == started_at)
        {
            registry.remove(meeting_id);
        }
    }

    /// Runs `job` to its end and records the outcome on its process row.
    pub(crate) async fn process_transcript_background<R: Runtime>(
        app: AppHandle<R>,
        job: SummaryJob,
    ) {
        info!(
            "Starting background processing for meeting_id: {}",
            job.meeting_id
        );
        let started = Utc::now();
        let outcome = tokio::select! {
            biased;
            () = job.cancellation.cancelled() => Err(Stopped::Cancelled),
            result = generate(&app, &job) => result.map_err(Stopped::Failed),
        };
        let seconds = (Utc::now() - started).as_seconds_f64();
        match outcome {
            Ok(generated) => complete(&job, generated, seconds).await,
            Err(Stopped::Cancelled) => cancelled(&job).await,
            Err(Stopped::Failed(message)) => failed(&job, &message).await,
        }
        Self::cleanup_cancellation_token(&job.meeting_id, job.started_at);
    }
}

fn registry() -> std::sync::MutexGuard<'static, HashMap<String, (DateTime<Utc>, CancellationToken)>>
{
    CANCELLATION_REGISTRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// The summary of `job`, through the gateway.
async fn generate<R: Runtime>(
    app: &AppHandle<R>,
    job: &SummaryJob,
) -> Result<GeneratedSummary, String> {
    let target = target(&job.provider, &job.model)?;
    let template = templates::get_template(&job.template_id)
        .map_err(|e| format!("Failed to load template '{}': {}", job.template_id, e))?;
    let generator = app
        .try_state::<Arc<SummaryGenerator>>()
        .ok_or("Summaries are not ready yet; try again in a moment")?;
    let request = SummaryRequest {
        transcript: job.text.clone(),
        user_context: Some(job.custom_prompt.clone()).filter(|context| !context.trim().is_empty()),
        section_instructions: template.to_section_instructions(),
        template_markdown: template.to_markdown_structure(),
        target,
    };
    generator
        .generate(&request)
        .await
        .map_err(|error| error.to_string())
}

/// The provider and model, from the names the UI and settings use.
fn target(provider: &str, model: &str) -> Result<Target, String> {
    let provider = match provider.to_lowercase().as_str() {
        "openai" => ProviderId::OpenAi,
        "claude" => ProviderId::Anthropic,
        "groq" => ProviderId::Groq,
        "ollama" => ProviderId::Ollama,
        "openrouter" => ProviderId::OpenRouter,
        "builtin-ai" | "local-llama" | "localllama" => ProviderId::BuiltIn,
        "custom-openai" => ProviderId::CustomOpenAi,
        other => return Err(format!("Unsupported LLM provider: {other}")),
    };
    if model.trim().is_empty() {
        return Err("No model is selected for summaries; choose one in Settings".to_string());
    }
    Ok(Target {
        provider,
        model: model.to_string(),
    })
}

async fn complete(job: &SummaryJob, generated: GeneratedSummary, seconds: f64) {
    let result = match build_summary_result_json(&generated.markdown, generated.reasoning_stripped)
    {
        Ok(result) => result,
        Err(message) => return failed(job, &message).await,
    };
    let chunks = i64::try_from(generated.chunks).unwrap_or(i64::MAX);
    let saved = SummaryProcessesRepository::update_process_completed(
        &job.pool,
        &job.meeting_id,
        job.started_at,
        result,
        chunks,
        seconds,
    )
    .await;
    match saved {
        Ok(true) => rename_meeting(job, &generated.markdown).await,
        Ok(false) => warn!(
            "Skipped stale summary completion for meeting_id: {}",
            job.meeting_id
        ),
        Err(e) => error!(
            "Failed to save completed process for {}: {}",
            job.meeting_id, e
        ),
    }
}

/// The summary's first heading becomes the meeting's name.
async fn rename_meeting(job: &SummaryJob, markdown: &str) {
    if let Some(name) = first_heading(markdown) {
        if let Err(e) =
            MeetingsRepository::update_meeting_name(&job.pool, &job.meeting_id, &name).await
        {
            error!(
                "Failed to update meeting name for {}: {}",
                job.meeting_id, e
            );
        }
    }
    info!(
        "Summary saved successfully for meeting_id: {}",
        job.meeting_id
    );
}

async fn cancelled(job: &SummaryJob) {
    match SummaryProcessesRepository::update_process_cancelled(
        &job.pool,
        &job.meeting_id,
        job.started_at,
    )
    .await
    {
        Ok(true) => info!(
            "Summary generation was cancelled for meeting_id: {}",
            job.meeting_id
        ),
        Ok(false) => warn!(
            "Skipped stale summary cancellation for meeting_id: {}",
            job.meeting_id
        ),
        Err(e) => error!(
            "Failed to update DB status to cancelled for {}: {}",
            job.meeting_id, e
        ),
    }
}

async fn failed(job: &SummaryJob, message: &str) {
    // The message is user-safe: stages, providers and counts, never content.
    error!(
        "Processing failed for meeting_id {}: {}",
        job.meeting_id, message
    );
    let saved = SummaryProcessesRepository::update_process_failed(
        &job.pool,
        &job.meeting_id,
        job.started_at,
        message,
    )
    .await;
    match saved {
        Ok(true) => {}
        Ok(false) => warn!(
            "Skipped stale summary failure for meeting_id: {}",
            job.meeting_id
        ),
        Err(e) => error!(
            "Failed to update DB status to failed for {}: {}",
            job.meeting_id, e
        ),
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
        assert!(SummaryService::cancel_summary(
            &meeting_id,
            second_started_at
        ));
        assert!(second.is_cancelled());
        SummaryService::cleanup_cancellation_token(&meeting_id, second_started_at);
    }

    #[test]
    fn legacy_provider_names_map_to_gateway_providers() {
        assert_eq!(
            target("claude", "claude-sonnet-5-5").unwrap().provider,
            ProviderId::Anthropic
        );
        assert_eq!(
            target("builtin-ai", "qwen3.5:2b").unwrap().provider,
            ProviderId::BuiltIn
        );
        assert_eq!(
            target("custom-openai", "llama").unwrap().provider,
            ProviderId::CustomOpenAi
        );
        assert!(target("gemini", "x").is_err());
        assert!(target("openai", " ").is_err());
    }
}
