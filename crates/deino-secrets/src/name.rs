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
    /// The client secret of the user's Google OAuth desktop client (ORG-01).
    #[serde(rename = "google_oauth_client_secret")]
    GoogleOAuthClientSecret,
    /// The refresh token of the connected Google Calendar account (ORG-01).
    #[serde(rename = "google_calendar_refresh_token")]
    GoogleCalendarRefreshToken,
    /// The refresh token of the connected Microsoft (Outlook) calendar
    /// account (ORG-01).
    #[serde(rename = "microsoft_calendar_refresh_token")]
    MicrosoftCalendarRefreshToken,
    /// A Granola personal API key, for importing notes (CAP-12).
    #[serde(rename = "granola_api_key")]
    GranolaApiKey,
    /// A Circleback API key, for importing meetings (CAP-12).
    #[serde(rename = "circleback_api_key")]
    CirclebackApiKey,
    /// A Fireflies API key, for importing transcripts (CAP-12).
    #[serde(rename = "fireflies_api_key")]
    FirefliesApiKey,
    /// A Fathom API key, for importing meetings (CAP-12).
    #[serde(rename = "fathom_api_key")]
    FathomApiKey,
}

impl SecretName {
    /// Every secret, in declaration order.
    pub const ALL: [Self; 13] = [
        Self::OpenAiApiKey,
        Self::AnthropicApiKey,
        Self::GroqApiKey,
        Self::OpenRouterApiKey,
        Self::OllamaApiKey,
        Self::CustomOpenAiApiKey,
        Self::GoogleOAuthClientSecret,
        Self::GoogleCalendarRefreshToken,
        Self::MicrosoftCalendarRefreshToken,
        Self::GranolaApiKey,
        Self::CirclebackApiKey,
        Self::FirefliesApiKey,
        Self::FathomApiKey,
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
            Self::GoogleOAuthClientSecret => "google_oauth_client_secret",
            Self::GoogleCalendarRefreshToken => "google_calendar_refresh_token",
            Self::MicrosoftCalendarRefreshToken => "microsoft_calendar_refresh_token",
            Self::GranolaApiKey => "granola_api_key",
            Self::CirclebackApiKey => "circleback_api_key",
            Self::FirefliesApiKey => "fireflies_api_key",
            Self::FathomApiKey => "fathom_api_key",
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
            Self::GoogleOAuthClientSecret => "Google client secret",
            Self::GoogleCalendarRefreshToken => "Google Calendar sign-in",
            Self::MicrosoftCalendarRefreshToken => "Microsoft calendar sign-in",
            Self::GranolaApiKey => "Granola API key",
            Self::CirclebackApiKey => "Circleback API key",
            Self::FirefliesApiKey => "Fireflies API key",
            Self::FathomApiKey => "Fathom API key",
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
