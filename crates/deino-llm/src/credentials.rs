//! Credential and endpoint ports: where providers find their API keys and
//! the URLs of user-configured servers.
//!
//! Port-first (PARITY_PLAN §7.0): the gateway depends on these traits, not on
//! a store. The app implements them over the legacy `settings` table until
//! FND-06's `deino-secrets` (Keychain) and settings store land; then only the
//! adapter changes. Keys never cross IPC: the UI sees [`ApiKey::masked`].

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use async_trait::async_trait;

use crate::ProviderId;

/// An API key. Its `Debug` output is redacted, so it cannot leak into logs.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    /// A key from user input; `None` when blank.
    pub fn new(key: impl Into<String>) -> Option<Self> {
        let key = key.into();
        let trimmed = key.trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_owned()))
    }

    /// The secret itself, for the one header that needs it.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// A display form that identifies the key without revealing it:
    /// `sk-a…7f3c`, or dots for keys too short to show any of.
    pub fn masked(&self) -> String {
        const SHOWN: usize = 4;
        let chars: Vec<char> = self.0.chars().collect();
        if chars.len() < SHOWN * 3 {
            return "••••••••".to_owned();
        }
        let head: String = chars.iter().take(SHOWN).collect();
        let tail: String = chars.iter().skip(chars.len() - SHOWN).collect();
        format!("{head}…{tail}")
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ApiKey(redacted)")
    }
}

/// A user-configured server: Ollama's address or a custom
/// OpenAI-compatible endpoint, with that endpoint's sampling overrides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Endpoint {
    /// The base URL, for example `http://localhost:11434` or `http://host:8000/v1`.
    pub base_url: String,
    /// Sampling the user set for this endpoint (custom endpoints only).
    pub sampling: SamplingOverrides,
}

/// Sampling parameters a user pinned for a custom endpoint.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SamplingOverrides {
    /// The answer's token cap, sent as `max_tokens`.
    pub max_tokens: Option<u32>,
    /// The sampling temperature.
    pub temperature: Option<f32>,
    /// Nucleus sampling.
    pub top_p: Option<f32>,
}

/// The credential store failed. Its message is the store's own user-safe
/// text (for example "enter the key again in Settings"); never a key.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct CredentialError(#[source] pub Box<dyn Error + Send + Sync>);

/// Where API keys live.
#[async_trait]
pub trait CredentialSource: Send + Sync {
    /// The key for `provider`, if the user set one.
    async fn api_key(&self, provider: ProviderId) -> Result<Option<ApiKey>, CredentialError>;
}

/// Where user-configured server addresses live.
#[async_trait]
pub trait EndpointSource: Send + Sync {
    /// The endpoint for `provider`, if the user configured one. Only Ollama
    /// and custom OpenAI-compatible providers have one.
    async fn endpoint(&self, provider: ProviderId) -> Result<Option<Endpoint>, CredentialError>;
}

/// Fixed keys and endpoints, for tests and the eval harness.
#[derive(Clone, Debug, Default)]
pub struct StaticCredentials {
    keys: BTreeMap<ProviderId, ApiKey>,
    endpoints: BTreeMap<ProviderId, Endpoint>,
}

impl StaticCredentials {
    /// No keys and no endpoints.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a key; a blank key is ignored, as the store would.
    #[must_use]
    pub fn key(mut self, provider: ProviderId, key: &str) -> Self {
        if let Some(key) = ApiKey::new(key) {
            self.keys.insert(provider, key);
        }
        self
    }

    /// Adds an endpoint with no sampling overrides.
    #[must_use]
    pub fn endpoint(mut self, provider: ProviderId, base_url: &str) -> Self {
        let endpoint = Endpoint {
            base_url: base_url.to_owned(),
            sampling: SamplingOverrides::default(),
        };
        self.endpoints.insert(provider, endpoint);
        self
    }
}

#[async_trait]
impl CredentialSource for StaticCredentials {
    async fn api_key(&self, provider: ProviderId) -> Result<Option<ApiKey>, CredentialError> {
        Ok(self.keys.get(&provider).cloned())
    }
}

#[async_trait]
impl EndpointSource for StaticCredentials {
    async fn endpoint(&self, provider: ProviderId) -> Result<Option<Endpoint>, CredentialError> {
        Ok(self.endpoints.get(&provider).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_trimmed_and_blank_keys_rejected() {
        assert_eq!(ApiKey::new("  sk-1  ").unwrap().expose(), "sk-1");
        assert!(ApiKey::new(" \n ").is_none());
    }

    #[test]
    fn debug_output_never_shows_the_key() {
        let key = ApiKey::new("sk-ant-secret-value").unwrap();

        assert_eq!(format!("{key:?}"), "ApiKey(redacted)");
    }

    #[test]
    fn masking_shows_only_the_ends_of_long_keys() {
        assert_eq!(
            ApiKey::new("sk-ant-api03-abcdef7f3c").unwrap().masked(),
            "sk-a…7f3c"
        );
        assert_eq!(ApiKey::new("short").unwrap().masked(), "••••••••");
    }
}
