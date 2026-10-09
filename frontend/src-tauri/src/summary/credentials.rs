//! What the legacy summary code needs to reach its LLM provider: the key
//! from the Keychain and the endpoints from the typed settings (FND-06).
//!
//! FND-09's gateway replaces this when it replaces `summary/`.

use deino_core::settings::{CustomOpenAiSettings, LlmProvider};
use tauri::{AppHandle, Runtime};

use crate::summary::llm_client::LLMProvider;

/// A provider's key and endpoints.
#[derive(Debug, Default)]
pub(crate) struct ProviderAccess {
    /// The key to send; empty when the provider needs none.
    pub api_key: String,
    /// The Ollama server, when the provider is Ollama and one is set.
    pub ollama_endpoint: Option<String>,
    /// The endpoint and sampling parameters, when the provider is custom.
    pub custom_openai: Option<CustomOpenAiSettings>,
}

/// Looks up how to reach `provider`. Fails with a user-facing message when a
/// required key is missing, or when macOS refuses to hand it over (the
/// "enter it again" case of ad-hoc builds).
pub(crate) async fn provider_access<R: Runtime>(
    app: &AppHandle<R>,
    provider: &LLMProvider,
) -> Result<ProviderAccess, String> {
    let ai = crate::features::settings::current(app).await.ai;
    match provider {
        LLMProvider::BuiltInAI => Ok(ProviderAccess::default()),
        // Ollama's key was never sent; requests go to the endpoint alone.
        LLMProvider::Ollama => Ok(ProviderAccess {
            ollama_endpoint: ai.ollama_endpoint,
            ..ProviderAccess::default()
        }),
        LLMProvider::CustomOpenAI => {
            let custom = ai.custom_openai.ok_or(
                "Custom OpenAI provider selected but no configuration found".to_owned(),
            )?;
            Ok(ProviderAccess {
                api_key: key(app, LlmProvider::CustomOpenAi, false).await?,
                custom_openai: Some(custom),
                ..ProviderAccess::default()
            })
        }
        cloud => Ok(ProviderAccess {
            api_key: key(app, settings_provider(cloud), true).await?,
            ..ProviderAccess::default()
        }),
    }
}

/// The saved key for `provider`, or an empty string when it may have none.
async fn key<R: Runtime>(
    app: &AppHandle<R>,
    provider: LlmProvider,
    required: bool,
) -> Result<String, String> {
    let Some(name) = provider.api_key() else {
        return Ok(String::new());
    };
    let stored = crate::features::secrets::stored_key(app, name)
        .await
        .map_err(|error| error.to_string())?;
    match stored {
        Some(value) => Ok(value.expose().to_owned()),
        None if required => Err(format!("No {} is saved; add it in Settings", name.label())),
        None => Ok(String::new()),
    }
}

/// The settings' name for a legacy provider.
fn settings_provider(provider: &LLMProvider) -> LlmProvider {
    match provider {
        LLMProvider::OpenAI => LlmProvider::OpenAi,
        LLMProvider::Claude => LlmProvider::Claude,
        LLMProvider::Groq => LlmProvider::Groq,
        LLMProvider::OpenRouter => LlmProvider::OpenRouter,
        LLMProvider::Ollama => LlmProvider::Ollama,
        LLMProvider::CustomOpenAI => LlmProvider::CustomOpenAi,
        LLMProvider::BuiltInAI => LlmProvider::BuiltInAi,
    }
}
