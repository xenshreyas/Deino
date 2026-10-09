//! [`LegacySqliteImport`]: moves the SQLite `settings` and
//! `transcript_settings` rows into [`Settings`] and the Keychain.
//!
//! It runs as a `deino_db::DataMigration` just before
//! [`DROP_LEGACY_SETTINGS_VERSION`] drops both tables, so it reads the
//! schema as it was then. The queries are runtime-checked (`query_as`, not
//! `query_as!`): the tables do not exist in the final schema that the
//! compile-time checks run against.

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::{DataMigration, DataMigrationError, Database};
use deino_secrets::{SecretName, SecretStore, SecretValue};

use super::super::{LlmProvider, SettingsService, TranscriptionEngine};
use super::apply_if_valid;
use super::formats::LegacyCustomOpenAi;
use crate::settings::Settings;

/// The version of `20261009061225_fnd06_drop_legacy_settings.sql`.
pub const DROP_LEGACY_SETTINGS_VERSION: i64 = 20_261_009_061_225;

/// The `settings` row: the summary model, endpoints and plaintext keys.
#[derive(Debug, Default, sqlx::FromRow)]
#[sqlx(rename_all = "camelCase")]
struct ModelRow {
    provider: String,
    model: String,
    openai_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    groq_api_key: Option<String>,
    open_router_api_key: Option<String>,
    ollama_api_key: Option<String>,
    ollama_endpoint: Option<String>,
    #[sqlx(rename = "customOpenAIConfig")]
    custom_openai_config: Option<String>,
}

/// The `transcript_settings` row: the engine, plus keys of retired cloud
/// engines, two of which double as LLM keys.
#[derive(Debug, Default, sqlx::FromRow)]
#[sqlx(rename_all = "camelCase")]
struct TranscriptRow {
    provider: String,
    model: String,
    openai_api_key: Option<String>,
    groq_api_key: Option<String>,
}

/// Moves the legacy SQLite settings before their tables are dropped.
pub struct LegacySqliteImport {
    settings: Arc<SettingsService>,
    secrets: Arc<dyn SecretStore>,
}

impl LegacySqliteImport {
    /// An import into `settings` and `secrets`.
    pub fn new(settings: Arc<SettingsService>, secrets: Arc<dyn SecretStore>) -> Self {
        Self { settings, secrets }
    }

    /// Saves each legacy key in the Keychain. A key that cannot be saved is
    /// logged by name and skipped: the user enters it again, which beats a
    /// database that never opens.
    async fn move_keys(&self, keys: Vec<(SecretName, String)>) {
        for (name, raw) in keys {
            let Ok(value) = SecretValue::parse(&raw) else {
                continue;
            };
            if let Err(error) = self.secrets.set(name, value).await {
                log::warn!(target: "settings", "could not move the legacy {} to the Keychain: {error}", name.label());
            }
        }
    }
}

#[async_trait]
impl DataMigration for LegacySqliteImport {
    fn runs_before(&self) -> i64 {
        DROP_LEGACY_SETTINGS_VERSION
    }

    async fn run(&self, database: &Database) -> Result<(), DataMigrationError> {
        let model = read_row::<ModelRow>(database, MODEL_ROW).await?;
        let transcript = read_row::<TranscriptRow>(database, TRANSCRIPT_ROW).await?;
        self.move_keys(legacy_keys(model.as_ref(), transcript.as_ref()))
            .await;
        self.settings
            .update(|settings| apply_rows(settings, model.as_ref(), transcript.as_ref()))
            .await?;
        log::info!(target: "settings", "moved the legacy SQLite settings");
        Ok(())
    }
}

const MODEL_ROW: &str = "SELECT provider, model, openaiApiKey, anthropicApiKey, groqApiKey, openRouterApiKey, ollamaApiKey, ollamaEndpoint, customOpenAIConfig FROM settings ORDER BY id = '1' DESC LIMIT 1";
const TRANSCRIPT_ROW: &str = "SELECT provider, model, openaiApiKey, groqApiKey FROM transcript_settings ORDER BY id = '1' DESC LIMIT 1";

async fn read_row<T>(database: &Database, query: &str) -> Result<Option<T>, sqlx::Error>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
{
    sqlx::query_as::<_, T>(query)
        .fetch_optional(database.writer())
        .await
}

/// Every non-empty legacy key with the secret it becomes. The transcript
/// row's OpenAI and Groq keys fill in when the model row has none.
fn legacy_keys(
    model: Option<&ModelRow>,
    transcript: Option<&TranscriptRow>,
) -> Vec<(SecretName, String)> {
    let from_model = |key: fn(&ModelRow) -> Option<&String>| model.and_then(key).cloned();
    let from_transcript =
        |key: fn(&TranscriptRow) -> Option<&String>| transcript.and_then(key).cloned();
    let custom = model
        .and_then(|row| row.custom_openai_config.as_deref())
        .and_then(LegacyCustomOpenAi::parse);
    [
        (
            SecretName::OpenAiApiKey,
            from_model(|row| row.openai_api_key.as_ref())
                .or_else(|| from_transcript(|row| row.openai_api_key.as_ref())),
        ),
        (
            SecretName::AnthropicApiKey,
            from_model(|row| row.anthropic_api_key.as_ref()),
        ),
        (
            SecretName::GroqApiKey,
            from_model(|row| row.groq_api_key.as_ref())
                .or_else(|| from_transcript(|row| row.groq_api_key.as_ref())),
        ),
        (
            SecretName::OpenRouterApiKey,
            from_model(|row| row.open_router_api_key.as_ref()),
        ),
        (
            SecretName::OllamaApiKey,
            from_model(|row| row.ollama_api_key.as_ref()),
        ),
        (
            SecretName::CustomOpenAiApiKey,
            custom.and_then(|config| config.api_key),
        ),
    ]
    .into_iter()
    .filter_map(|(name, key)| Some((name, key?)))
    .filter(|(_, key)| !key.trim().is_empty())
    .collect()
}

/// Copies the non-secret parts of the legacy rows into `settings`.
fn apply_rows(
    settings: &mut Settings,
    model: Option<&ModelRow>,
    transcript: Option<&TranscriptRow>,
) {
    if let Some(row) = model {
        apply_model(settings, row);
    }
    if let Some(engine) = transcript.and_then(|row| parse::<TranscriptionEngine>(&row.provider)) {
        let model = transcript.map(|row| row.model.clone()).unwrap_or_default();
        apply_if_valid(settings, "transcription", |s| {
            s.transcription.engine = engine;
            if !model.trim().is_empty() {
                s.transcription.model = model;
            }
        });
    }
}

fn apply_model(settings: &mut Settings, row: &ModelRow) {
    if let Some(provider) = parse::<LlmProvider>(&row.provider) {
        settings.ai.provider = provider;
        if !row.model.trim().is_empty() {
            settings.ai.model.clone_from(&row.model);
            settings
                .ai
                .provider_models
                .insert(provider, row.model.clone());
        }
    }
    let endpoint = row
        .ollama_endpoint
        .clone()
        .filter(|url| !url.trim().is_empty());
    apply_if_valid(settings, "Ollama endpoint", |s| {
        s.ai.ollama_endpoint = endpoint;
    });
    let custom = row
        .custom_openai_config
        .as_deref()
        .and_then(LegacyCustomOpenAi::parse);
    if let Some(custom) = custom {
        apply_if_valid(settings, "custom endpoint", |s| {
            s.ai.custom_openai = Some(custom.into_settings());
        });
    }
}

/// Parses a legacy enum name with the type's serde names; `None` if unknown.
fn parse<T: serde::de::DeserializeOwned>(name: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(name.to_owned())).ok()
}

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod tests;
