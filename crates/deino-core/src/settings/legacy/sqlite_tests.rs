//! The SQLite import against real legacy rows: what moves where, and that no
//! key is left in plaintext in the database file or the settings file.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use deino_db::{DataMigration, DataMigrationError, Database, DbLocation, MIGRATOR};
use deino_secrets::{InMemorySecretStore, SecretName};

use super::*;
use crate::settings::{
    DEFAULT_PARAKEET_MODEL, JsonFileSettingsStore, RecordedSettingsEvents, SettingsService,
    TranscriptionEngine,
};

const OPENAI_KEY: &str = "sk-legacy-openai-0123456789";
const CLAUDE_KEY: &str = "sk-ant-legacy-0123456789";
const CUSTOM_KEY: &str = "sk-custom-legacy-0123456789";
const TRANSCRIPT_GROQ_KEY: &str = "gsk-transcript-legacy-0123456789";

/// Writes legacy rows into the schema the import reads, just before it runs.
struct Seed(Vec<String>);

#[async_trait]
impl DataMigration for Seed {
    fn runs_before(&self) -> i64 {
        DROP_LEGACY_SETTINGS_VERSION
    }

    async fn run(&self, database: &Database) -> Result<(), DataMigrationError> {
        for statement in &self.0 {
            sqlx::query(statement).execute(database.writer()).await?;
        }
        Ok(())
    }
}

/// A model row as the legacy app wrote it, with OpenAI, Claude and custom keys.
fn model_row(provider: &str, model: &str, ollama_endpoint: &str) -> String {
    let custom = format!(
        r#"{{"endpoint":"http://localhost:8000/v1","apiKey":"{CUSTOM_KEY}","model":"llama-3","maxTokens":512,"temperature":0.4,"topP":null}}"#
    );
    format!(
        "INSERT INTO settings (id, provider, model, whisperModel, openaiApiKey, anthropicApiKey, ollamaEndpoint, customOpenAIConfig) \
         VALUES ('1', '{provider}', '{model}', 'large-v3', '{OPENAI_KEY}', '{CLAUDE_KEY}', '{ollama_endpoint}', '{custom}')"
    )
}

fn transcript_row(provider: &str, model: &str) -> String {
    format!(
        "INSERT INTO transcript_settings (id, provider, model, groqApiKey) VALUES ('1', '{provider}', '{model}', '{TRANSCRIPT_GROQ_KEY}')"
    )
}

struct Imported {
    settings: Arc<SettingsService>,
    secrets: Arc<InMemorySecretStore>,
    directory: tempfile::TempDir,
}

/// Opens a file database seeded with `rows`, importing into a settings file.
async fn import(rows: Vec<String>) -> Imported {
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(JsonFileSettingsStore::new(
        directory.path().join("settings.json"),
    ));
    let events = Arc::new(RecordedSettingsEvents::default());
    let settings = Arc::new(SettingsService::load(store, events).await.unwrap());
    let secrets = Arc::new(InMemorySecretStore::default());
    let import = LegacySqliteImport::new(settings.clone(), secrets.clone());
    let location = DbLocation::File(directory.path().join("deino.sqlite"));
    let seed = Seed(rows);
    let steps: [&dyn DataMigration; 2] = [&seed, &import];
    let database = Database::open_with(&location, &MIGRATOR, &steps);
    database.await.unwrap().close().await;
    Imported {
        settings,
        secrets,
        directory,
    }
}

fn contains(path: &Path, needle: &str) -> bool {
    std::fs::read(path).is_ok_and(|bytes| {
        bytes
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    })
}

#[test]
fn the_import_runs_before_the_migration_that_drops_the_tables() {
    assert!(MIGRATOR.version_exists(DROP_LEGACY_SETTINGS_VERSION));
}

#[tokio::test]
async fn keys_move_to_the_keychain_and_no_plaintext_copy_remains() {
    let rows = vec![
        model_row("openai", "gpt-4o", ""),
        transcript_row("parakeet", ""),
    ];

    let Imported {
        secrets, directory, ..
    } = import(rows).await;

    assert_eq!(
        secrets.stored(SecretName::OpenAiApiKey).as_deref(),
        Some(OPENAI_KEY)
    );
    assert_eq!(
        secrets.stored(SecretName::AnthropicApiKey).as_deref(),
        Some(CLAUDE_KEY)
    );
    assert_eq!(
        secrets.stored(SecretName::CustomOpenAiApiKey).as_deref(),
        Some(CUSTOM_KEY)
    );
    assert_eq!(
        secrets.stored(SecretName::GroqApiKey).as_deref(),
        Some(TRANSCRIPT_GROQ_KEY)
    );
    for key in [OPENAI_KEY, CLAUDE_KEY, CUSTOM_KEY, TRANSCRIPT_GROQ_KEY] {
        for file in ["deino.sqlite", "deino.sqlite-wal", "settings.json"] {
            let path = directory.path().join(file);
            assert!(
                !contains(&path, key),
                "a key is still in plaintext in {file}"
            );
        }
    }
}

#[tokio::test]
async fn model_endpoint_and_transcription_settings_are_carried_over() {
    let rows = vec![
        model_row("claude", "claude-sonnet-4-5", "http://mac-mini:11434"),
        transcript_row("localWhisper", "large-v3-turbo"),
    ];

    let settings = import(rows).await.settings.get().await;

    assert_eq!(settings.ai.provider, LlmProvider::Claude);
    assert_eq!(settings.ai.model, "claude-sonnet-4-5");
    assert_eq!(
        settings.ai.provider_models[&LlmProvider::Claude],
        "claude-sonnet-4-5"
    );
    assert_eq!(
        settings.ai.ollama_endpoint.as_deref(),
        Some("http://mac-mini:11434")
    );
    let custom = settings.ai.custom_openai.unwrap();
    assert_eq!(
        (custom.model.as_str(), custom.max_tokens),
        ("llama-3", Some(512))
    );
    assert_eq!(settings.transcription.engine, TranscriptionEngine::Whisper);
    assert_eq!(settings.transcription.model, "large-v3-turbo");
}

#[tokio::test]
async fn a_legacy_parakeet_choice_keeps_its_engine_and_model() {
    let rows = vec![transcript_row("parakeet", "")];

    let settings = import(rows).await.settings.get().await;

    assert_eq!(settings.transcription.engine, TranscriptionEngine::Parakeet);
    assert_eq!(settings.transcription.model, DEFAULT_PARAKEET_MODEL);
}

#[tokio::test]
async fn a_legacy_value_that_breaks_a_rule_is_skipped_not_fatal() {
    let rows = vec![model_row("ollama", "llama3.2", "mac-mini:11434")];

    let Imported {
        settings, secrets, ..
    } = import(rows).await;

    let settings = settings.get().await;
    assert_eq!(settings.ai.provider, LlmProvider::Ollama);
    assert_eq!(settings.ai.ollama_endpoint, None);
    assert!(secrets.stored(SecretName::OpenAiApiKey).is_some());
}

#[tokio::test]
async fn a_database_without_legacy_rows_changes_nothing() {
    let Imported {
        settings,
        secrets,
        directory,
    } = import(Vec::new()).await;

    assert_eq!(settings.get().await, Settings::default());
    assert!(secrets.names().is_empty());
    assert!(!directory.path().join("settings.json").exists());
}
