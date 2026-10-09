//! The LLM providers Deino can talk to, as a closed set.

use std::fmt;

use serde::{Deserialize, Serialize};

/// An LLM provider. Every call names one, through a route or a pinned target.
///
/// Stored as TEXT in `llm_calls.provider` and the route settings, with the
/// serialized names below; renaming a variant needs a migration.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    specta::Type,
    sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ProviderId {
    /// Anthropic's Messages API (Claude).
    Anthropic,
    /// OpenAI's Chat Completions API.
    #[serde(rename = "openai")]
    #[sqlx(rename = "openai")]
    OpenAi,
    /// Groq's OpenAI-compatible API.
    Groq,
    /// OpenRouter's OpenAI-compatible API.
    #[serde(rename = "openrouter")]
    #[sqlx(rename = "openrouter")]
    OpenRouter,
    /// Any OpenAI-compatible endpoint the user configures.
    #[serde(rename = "custom_openai")]
    #[sqlx(rename = "custom_openai")]
    CustomOpenAi,
    /// A local or remote Ollama server.
    Ollama,
    /// The built-in local model, run by the llama-helper sidecar.
    #[serde(rename = "builtin")]
    #[sqlx(rename = "builtin")]
    BuiltIn,
}

impl ProviderId {
    /// Every provider, in the order Settings → AI lists them.
    pub const ALL: [Self; 7] = [
        Self::Anthropic,
        Self::OpenAi,
        Self::Groq,
        Self::OpenRouter,
        Self::CustomOpenAi,
        Self::Ollama,
        Self::BuiltIn,
    ];

    /// The name people know the provider by.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::OpenAi => "OpenAI",
            Self::Groq => "Groq",
            Self::OpenRouter => "OpenRouter",
            Self::CustomOpenAi => "Custom OpenAI-compatible",
            Self::Ollama => "Ollama",
            Self::BuiltIn => "Built-in",
        }
    }

    /// Whether the provider refuses requests without an API key. Ollama and
    /// custom endpoints accept an optional key; the built-in model needs none.
    pub fn requires_api_key(self) -> bool {
        matches!(
            self,
            Self::Anthropic | Self::OpenAi | Self::Groq | Self::OpenRouter
        )
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_serialize_with_their_stored_names() {
        let names: Vec<String> = ProviderId::ALL
            .iter()
            .map(|provider| serde_json::to_value(provider).unwrap())
            .map(|value| value.as_str().unwrap().to_owned())
            .collect();

        assert_eq!(
            names,
            [
                "anthropic",
                "openai",
                "groq",
                "openrouter",
                "custom_openai",
                "ollama",
                "builtin"
            ]
        );
    }

    #[test]
    fn only_hosted_providers_require_a_key() {
        let required: Vec<ProviderId> = ProviderId::ALL
            .into_iter()
            .filter(|provider| provider.requires_api_key())
            .collect();

        assert_eq!(
            required,
            [
                ProviderId::Anthropic,
                ProviderId::OpenAi,
                ProviderId::Groq,
                ProviderId::OpenRouter
            ]
        );
    }
}
