//! Where a connector's OAuth client comes from: the user's own app
//! registration, its client ID in the settings (`calendar.google.clientId`)
//! and Google's client secret in the Keychain.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::settings::SettingsService;
use deino_domain::CalendarProvider;
use deino_integrations::{OAuthClient, OAuthSpec};
use deino_secrets::{SecretName, SecretStore};

/// The user has not registered an app with the provider yet.
#[derive(Debug, thiserror::Error)]
#[error("set up {0} first: paste its client ID in Settings → Calendar")]
pub struct NotConfigured(pub &'static str);

/// Builds a provider's OAuth client from the user's registration.
#[async_trait]
pub trait Credentials: Send + Sync {
    /// The client for `provider`, whose OAuth rules are `spec`; fails until
    /// the user has set it up.
    async fn client(
        &self,
        provider: CalendarProvider,
        spec: &OAuthSpec,
    ) -> Result<OAuthClient, NotConfigured>;
}

/// The provider's name in messages.
pub(crate) const fn provider_name(provider: CalendarProvider) -> &'static str {
    match provider {
        CalendarProvider::Google => "Google",
        CalendarProvider::Microsoft => "Microsoft",
        CalendarProvider::Apple => "macOS Calendar",
    }
}

/// [`Credentials`] over the settings and the Keychain.
pub struct SettingsCredentials {
    settings: Arc<SettingsService>,
    secrets: Arc<dyn SecretStore>,
}

impl SettingsCredentials {
    /// Credentials read from `settings` and `secrets`.
    pub fn new(settings: Arc<SettingsService>, secrets: Arc<dyn SecretStore>) -> Self {
        Self { settings, secrets }
    }

    /// Google's client secret, when the provider needs one.
    async fn client_secret(
        &self,
        provider: CalendarProvider,
        spec: &OAuthSpec,
    ) -> Result<Option<deino_secrets::SecretValue>, NotConfigured> {
        if !spec.client_auth.needs_secret() {
            return Ok(None);
        }
        let name = provider_name(provider);
        let secret = self
            .secrets
            .get(SecretName::GoogleOAuthClientSecret)
            .await
            .map_err(|error| {
                log::warn!(target: "oauth", "could not read the {name} client secret: {error}");
                NotConfigured(name)
            })?
            .ok_or(NotConfigured(name))?;
        Ok(Some(secret))
    }
}

#[async_trait]
impl Credentials for SettingsCredentials {
    async fn client(
        &self,
        provider: CalendarProvider,
        spec: &OAuthSpec,
    ) -> Result<OAuthClient, NotConfigured> {
        let calendar = self.settings.get().await.calendar;
        let name = provider_name(provider);
        let connector = match provider {
            CalendarProvider::Google => &calendar.google,
            CalendarProvider::Microsoft => &calendar.microsoft,
            CalendarProvider::Apple => return Err(NotConfigured(name)),
        };
        let client_id = connector.client_id().ok_or(NotConfigured(name))?.to_owned();
        let client_secret = self.client_secret(provider, spec).await?;
        Ok(OAuthClient {
            client_id,
            client_secret,
        })
    }
}
