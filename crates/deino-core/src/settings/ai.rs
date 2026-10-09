//! The AI section: which LLM writes notes, and how to reach it.
//!
//! API keys are not here: they live in the Keychain under the
//! [`SecretName`] that [`LlmProvider::api_key`] names.

use std::collections::BTreeMap;

use deino_secrets::SecretName;
use serde::{Deserialize, Serialize};

use super::SettingsError;

/// An LLM provider. Serialized names match the ones the app has always used.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    specta::Type,
)]
pub enum LlmProvider {
    /// The built-in local model (llama-helper); needs no key.
    #[default]
    #[serde(rename = "builtin-ai")]
    BuiltInAi,
    /// OpenAI.
    #[serde(rename = "openai")]
    OpenAi,
    /// Anthropic's Claude.
    #[serde(rename = "claude")]
    Claude,
    /// Groq.
    #[serde(rename = "groq")]
    Groq,
    /// OpenRouter.
    #[serde(rename = "openrouter")]
    OpenRouter,
    /// An Ollama server, local by default.
    #[serde(rename = "ollama")]
    Ollama,
    /// Any OpenAI-compatible endpoint the user configures.
    #[serde(rename = "custom-openai")]
    CustomOpenAi,
}

impl LlmProvider {
    /// The Keychain secret holding this provider's key, if it can have one.
    pub const fn api_key(self) -> Option<SecretName> {
        match self {
            Self::BuiltInAi => None,
            Self::OpenAi => Some(SecretName::OpenAiApiKey),
            Self::Claude => Some(SecretName::AnthropicApiKey),
            Self::Groq => Some(SecretName::GroqApiKey),
            Self::OpenRouter => Some(SecretName::OpenRouterApiKey),
            Self::Ollama => Some(SecretName::OllamaApiKey),
            Self::CustomOpenAi => Some(SecretName::CustomOpenAiApiKey),
        }
    }

    /// True when requests fail without a key (cloud providers).
    pub const fn requires_api_key(self) -> bool {
        matches!(
            self,
            Self::OpenAi | Self::Claude | Self::Groq | Self::OpenRouter
        )
    }
}

/// The AI section of [`super::Settings`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AiSettings {
    /// The provider that writes notes.
    pub provider: LlmProvider,
    /// The provider's model, for example `gpt-4o` or `qwen3.5:2b`; empty
    /// until the user or first-run setup picks one.
    pub model: String,
    /// The model last chosen for each provider, restored when the user
    /// switches back to it.
    pub provider_models: BTreeMap<LlmProvider, String>,
    /// The Ollama server's URL; `None` means the local default.
    pub ollama_endpoint: Option<String>,
    /// The custom OpenAI-compatible endpoint, once configured.
    pub custom_openai: Option<CustomOpenAiSettings>,
    /// Generate notes automatically when a recording stops.
    pub auto_summary: bool,
}

impl AiSettings {
    /// Checks the endpoint URLs and the custom endpoint's parameters.
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        if let Some(endpoint) = &self.ollama_endpoint {
            require_http_url("ai.ollamaEndpoint", endpoint)?;
        }
        self.custom_openai
            .as_ref()
            .map_or(Ok(()), CustomOpenAiSettings::validate)
    }
}

/// An OpenAI-compatible endpoint. Its key is the Keychain secret
/// [`SecretName::CustomOpenAiApiKey`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct CustomOpenAiSettings {
    /// The API's base URL, for example `http://localhost:8000/v1`.
    pub endpoint: String,
    /// The model to request.
    pub model: String,
    /// The completion's token limit; the server's default when `None`.
    pub max_tokens: Option<u32>,
    /// Sampling temperature, 0 to 2.
    pub temperature: Option<f32>,
    /// Nucleus sampling, 0 to 1.
    pub top_p: Option<f32>,
}

impl CustomOpenAiSettings {
    fn validate(&self) -> Result<(), SettingsError> {
        require_http_url("ai.customOpenai.endpoint", &self.endpoint)?;
        if self.model.trim().is_empty() {
            return Err(SettingsError::invalid(
                "ai.customOpenai.model",
                "cannot be empty",
            ));
        }
        if self.max_tokens == Some(0) {
            return Err(SettingsError::invalid(
                "ai.customOpenai.maxTokens",
                "must be at least 1",
            ));
        }
        require_range("ai.customOpenai.temperature", self.temperature, 2.0)?;
        require_range("ai.customOpenai.topP", self.top_p, 1.0)
    }
}

/// Fails unless `value` is an `http://` or `https://` URL with a host.
fn require_http_url(setting: &'static str, value: &str) -> Result<(), SettingsError> {
    let host = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"));
    match host {
        Some(rest) if !rest.trim().is_empty() => Ok(()),
        _ => Err(SettingsError::invalid(
            setting,
            "must be an http:// or https:// URL",
        )),
    }
}

/// Fails unless `value` is absent or within `0..=max`.
fn require_range(setting: &'static str, value: Option<f32>, max: f32) -> Result<(), SettingsError> {
    match value {
        Some(number) if !(0.0..=max).contains(&number) => Err(SettingsError::invalid(
            setting,
            "is outside its allowed range",
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(endpoint: &str) -> CustomOpenAiSettings {
        CustomOpenAiSettings {
            endpoint: endpoint.into(),
            model: "llama-3".into(),
            ..CustomOpenAiSettings::default()
        }
    }

    #[test]
    fn providers_keep_their_legacy_names() {
        let names: Vec<_> = [LlmProvider::BuiltInAi, LlmProvider::CustomOpenAi]
            .iter()
            .map(|provider| serde_json::to_value(provider).unwrap())
            .collect();
        assert_eq!(names, ["builtin-ai", "custom-openai"]);
    }

    #[test]
    fn only_cloud_providers_require_a_key_and_the_local_model_has_none() {
        assert!(LlmProvider::Claude.requires_api_key());
        assert!(!LlmProvider::Ollama.requires_api_key());
        assert_eq!(LlmProvider::BuiltInAi.api_key(), None);
        assert_eq!(
            LlmProvider::Claude.api_key(),
            Some(SecretName::AnthropicApiKey)
        );
    }

    #[test]
    fn endpoints_must_be_http_urls() {
        let ai = AiSettings {
            ollama_endpoint: Some("localhost:11434".into()),
            ..AiSettings::default()
        };
        assert!(ai.validate().is_err());
        assert!(custom("https://api.example.com/v1").validate().is_ok());
        assert!(custom("http://").validate().is_err());
    }

    #[test]
    fn custom_parameters_are_range_checked() {
        let hot = CustomOpenAiSettings {
            temperature: Some(2.5),
            ..custom("http://localhost:8000")
        };
        let no_tokens = CustomOpenAiSettings {
            max_tokens: Some(0),
            ..custom("http://localhost:8000")
        };
        assert!(hot.validate().is_err());
        assert!(no_tokens.validate().is_err());
    }
}
