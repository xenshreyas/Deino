//! The secrets Deino knows: a closed set, so a typo cannot create a new one.

use serde::{Deserialize, Serialize};

/// A secret Deino keeps in the keychain: one per provider credential.
///
/// Each serializes to its keychain account name, for example
/// `"openai_api_key"`. A card that adds a credential (an integration token,
/// say) adds a variant here.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
pub enum SecretName {
    /// The OpenAI API key.
    #[serde(rename = "openai_api_key")]
    OpenAiApiKey,
    /// The Anthropic (Claude) API key.
    #[serde(rename = "anthropic_api_key")]
    AnthropicApiKey,
    /// The Groq API key.
    #[serde(rename = "groq_api_key")]
    GroqApiKey,
    /// The OpenRouter API key.
    #[serde(rename = "openrouter_api_key")]
    OpenRouterApiKey,
    /// The key of a remote Ollama server, if it needs one.
    #[serde(rename = "ollama_api_key")]
    OllamaApiKey,
    /// The key of the user's custom OpenAI-compatible endpoint.
    #[serde(rename = "custom_openai_api_key")]
    CustomOpenAiApiKey,
}

impl SecretName {
    /// Every secret, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::OpenAiApiKey,
        Self::AnthropicApiKey,
        Self::GroqApiKey,
        Self::OpenRouterApiKey,
        Self::OllamaApiKey,
        Self::CustomOpenAiApiKey,
    ];

    /// The keychain account the secret is stored under; equal to its serialized name.
    pub const fn account(self) -> &'static str {
        match self {
            Self::OpenAiApiKey => "openai_api_key",
            Self::AnthropicApiKey => "anthropic_api_key",
            Self::GroqApiKey => "groq_api_key",
            Self::OpenRouterApiKey => "openrouter_api_key",
            Self::OllamaApiKey => "ollama_api_key",
            Self::CustomOpenAiApiKey => "custom_openai_api_key",
        }
    }

    /// A user-facing name for messages, for example "OpenAI API key".
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenAiApiKey => "OpenAI API key",
            Self::AnthropicApiKey => "Anthropic API key",
            Self::GroqApiKey => "Groq API key",
            Self::OpenRouterApiKey => "OpenRouter API key",
            Self::OllamaApiKey => "Ollama API key",
            Self::CustomOpenAiApiKey => "custom endpoint API key",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn the_account_is_the_serialized_name() {
        for name in SecretName::ALL {
            let json = serde_json::to_value(name).unwrap();
            assert_eq!(json.as_str(), Some(name.account()), "{name:?}");
        }
    }

    #[test]
    fn accounts_are_unique() {
        let accounts: HashSet<_> = SecretName::ALL.iter().map(|name| name.account()).collect();
        assert_eq!(accounts.len(), SecretName::ALL.len());
    }
}
