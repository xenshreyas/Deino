//! Provider adapters: each turns the provider-agnostic request into its wire
//! format and its stream back into [`CompletionEvent`](crate::CompletionEvent)s.
//!
//! | Adapter | Serves |
//! |---|---|
//! | [`AnthropicProvider`] | Claude, Messages API |
//! | [`OpenAiCompatProvider`] | OpenAI, Groq, OpenRouter, custom endpoints |
//! | [`OllamaProvider`] | Ollama (chat over `/v1`, models and pulls over its native API) |
//! | [`BuiltInProvider`] | The llama-helper sidecar, through the [`LocalModels`] port |
//!
//! Adding a provider is a module here, a [`ProviderId`] variant, and one
//! registration in the app's composition root.

pub mod anthropic;
pub mod builtin;
pub mod ollama;
pub mod openai_compat;
pub(crate) mod status;
pub(crate) mod stream;

use serde::de::DeserializeOwned;

pub use self::anthropic::AnthropicProvider;
pub use self::builtin::{
    BuiltInProvider, EmbedReply, HelperCompletion, HelperEvent, HelperEventStream, HelperRequest,
    HelperSampling, HelperStop, LocalGeneration, LocalModels,
};
pub use self::ollama::{OllamaProvider, PullProgress};
pub use self::openai_compat::{Flavor, OpenAiCompatProvider};
use crate::{ApiKey, CredentialSource, LlmError, ProviderId};

/// The key for `provider`; [`LlmError::NoKey`] when none is set.
pub(crate) async fn required_key(
    credentials: &dyn CredentialSource,
    provider: ProviderId,
) -> Result<ApiKey, LlmError> {
    optional_key(credentials, provider)
        .await?
        .ok_or(LlmError::NoKey(provider))
}

/// The key for `provider`, if the user set one.
pub(crate) async fn optional_key(
    credentials: &dyn CredentialSource,
    provider: ProviderId,
) -> Result<Option<ApiKey>, LlmError> {
    credentials
        .api_key(provider)
        .await
        .map_err(|source| LlmError::Credentials { provider, source })
}

/// Parses a whole JSON response body, reporting only the error class.
pub(crate) fn parse_body<T: DeserializeOwned>(
    provider: ProviderId,
    body: &[u8],
) -> Result<T, LlmError> {
    serde_json::from_slice(body).map_err(|error| LlmError::Malformed {
        provider,
        detail: format!("unreadable response ({})", stream::category(&error)),
    })
}
