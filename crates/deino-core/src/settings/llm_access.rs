//! The LLM gateway's ports over the settings and the Keychain.
//!
//! [`LlmAccess`] implements `deino_llm`'s [`CredentialSource`] (keys from
//! the Keychain, under each provider's [`SecretName`]), [`EndpointSource`]
//! (`ai.ollamaEndpoint` and `ai.customOpenai`) and [`RouteStore`]
//! (`ai.routes`), so the gateway never reads a settings file or the
//! Keychain itself.

use std::sync::Arc;

use async_trait::async_trait;
use deino_llm::{
    ApiKey, CredentialError, CredentialSource, Endpoint, EndpointSource, ProviderId,
    RouteOverrides, RouteStore, RouteStoreError, SamplingOverrides,
};
use deino_secrets::{SecretName, SecretStore};

use super::{CustomOpenAiSettings, SettingsService};

/// Keys, endpoints and route choices for the LLM gateway.
pub struct LlmAccess {
    settings: Arc<SettingsService>,
    secrets: Arc<dyn SecretStore>,
}

impl LlmAccess {
    /// Access over the settings service and the secret store.
    pub fn new(settings: Arc<SettingsService>, secrets: Arc<dyn SecretStore>) -> Self {
        Self { settings, secrets }
    }
}

/// The Keychain secret that holds `provider`'s key, if it can have one.
pub fn secret_for(provider: ProviderId) -> Option<SecretName> {
    match provider {
        ProviderId::Anthropic => Some(SecretName::AnthropicApiKey),
        ProviderId::OpenAi => Some(SecretName::OpenAiApiKey),
        ProviderId::Groq => Some(SecretName::GroqApiKey),
        ProviderId::OpenRouter => Some(SecretName::OpenRouterApiKey),
        ProviderId::CustomOpenAi => Some(SecretName::CustomOpenAiApiKey),
        ProviderId::Ollama => Some(SecretName::OllamaApiKey),
        ProviderId::BuiltIn => None,
    }
}

#[async_trait]
impl CredentialSource for LlmAccess {
    async fn api_key(&self, provider: ProviderId) -> Result<Option<ApiKey>, CredentialError> {
        let Some(name) = secret_for(provider) else {
            return Ok(None);
        };
        let value = self
            .secrets
            .get(name)
            .await
            .map_err(|error| CredentialError(Box::new(error)))?;
        Ok(value.and_then(|value| ApiKey::new(value.expose())))
    }
}

#[async_trait]
impl EndpointSource for LlmAccess {
    async fn endpoint(&self, provider: ProviderId) -> Result<Option<Endpoint>, CredentialError> {
        let ai = self.settings.get().await.ai;
        Ok(match provider {
            ProviderId::Ollama => ai.ollama_endpoint.map(|base_url| Endpoint {
                base_url,
                sampling: SamplingOverrides::default(),
            }),
            ProviderId::CustomOpenAi => ai.custom_openai.map(custom_endpoint),
            _ => None,
        })
    }
}

fn custom_endpoint(custom: CustomOpenAiSettings) -> Endpoint {
    Endpoint {
        base_url: custom.endpoint,
        sampling: SamplingOverrides {
            max_tokens: custom.max_tokens,
            temperature: custom.temperature,
            top_p: custom.top_p,
        },
    }
}

#[async_trait]
impl RouteStore for LlmAccess {
    async fn load(&self) -> Result<RouteOverrides, RouteStoreError> {
        Ok(self.settings.get().await.ai.routes.into())
    }

    async fn save(&self, overrides: &RouteOverrides) -> Result<(), RouteStoreError> {
        let routes = overrides.clone().into();
        self.settings
            .update(move |settings| settings.ai.routes = routes)
            .await
            .map(drop)
            .map_err(|error| RouteStoreError(Box::new(error)))
    }
}

#[cfg(test)]
mod tests {
    use deino_llm::{Route, Target};
    use deino_secrets::{InMemorySecretStore, SecretValue};

    use super::*;
    use crate::settings::{FakeSettingsStore, RecordedSettingsEvents};

    async fn access() -> (LlmAccess, Arc<SettingsService>, Arc<InMemorySecretStore>) {
        let store = Arc::new(FakeSettingsStore::with(None));
        let events = Arc::new(RecordedSettingsEvents::default());
        let settings = Arc::new(SettingsService::load(store, events).await.unwrap());
        let secrets = Arc::new(InMemorySecretStore::default());
        (
            LlmAccess::new(settings.clone(), secrets.clone()),
            settings,
            secrets,
        )
    }

    #[tokio::test]
    async fn keys_come_from_the_keychain_under_each_provider_s_secret() {
        let (access, _, secrets) = access().await;
        secrets
            .set(
                SecretName::AnthropicApiKey,
                SecretValue::parse("sk-ant-1").unwrap(),
            )
            .await
            .unwrap();

        let claude = access.api_key(ProviderId::Anthropic).await.unwrap();
        let groq = access.api_key(ProviderId::Groq).await.unwrap();
        let local = access.api_key(ProviderId::BuiltIn).await.unwrap();

        assert_eq!(claude.unwrap().expose(), "sk-ant-1");
        assert_eq!((groq, local), (None, None));
    }

    #[tokio::test]
    async fn a_denied_keychain_read_is_a_credential_error_that_says_what_to_do() {
        let (access, _, secrets) = access().await;
        secrets
            .set(
                SecretName::OpenAiApiKey,
                SecretValue::parse("sk-1").unwrap(),
            )
            .await
            .unwrap();
        secrets.deny(SecretName::OpenAiApiKey);

        let error = access.api_key(ProviderId::OpenAi).await.unwrap_err();

        assert!(error.to_string().contains("enter it again"), "{error}");
    }

    #[tokio::test]
    async fn endpoints_come_from_the_ai_settings() {
        let (access, settings, _) = access().await;
        settings
            .update(|settings| {
                settings.ai.ollama_endpoint = Some("http://gpu-box:11434".into());
                settings.ai.custom_openai = Some(CustomOpenAiSettings {
                    endpoint: "http://host:8000/v1".into(),
                    model: "llama-3".into(),
                    max_tokens: Some(512),
                    temperature: Some(0.2),
                    top_p: None,
                });
            })
            .await
            .unwrap();

        let ollama = access.endpoint(ProviderId::Ollama).await.unwrap().unwrap();
        let custom = access
            .endpoint(ProviderId::CustomOpenAi)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(ollama.base_url, "http://gpu-box:11434");
        assert_eq!(custom.sampling.max_tokens, Some(512));
        assert_eq!(access.endpoint(ProviderId::OpenAi).await.unwrap(), None);
    }

    #[tokio::test]
    async fn route_choices_round_trip_through_the_settings() {
        let (access, settings, _) = access().await;
        let mut overrides = RouteOverrides::default();
        let target = Target {
            provider: ProviderId::OpenAi,
            model: "gpt-5".into(),
            effort: None,
        };
        overrides.set(Route::Chat, Some(target.clone()));

        access.save(&overrides).await.unwrap();

        assert_eq!(access.load().await.unwrap(), overrides);
        assert_eq!(
            settings.get().await.ai.routes.get(&Route::Chat),
            Some(&target)
        );
    }
}
