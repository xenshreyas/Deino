//! Where a connector's OAuth client comes from: the user's own app
//! registration, its client ID in the settings (`calendar.google.clientId`)
//! and Google's client secret in the Keychain.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::settings::SettingsService;
use deino_domain::CalendarProvider;
use deino_secrets::{SecretName, SecretStore};

use crate::oauth::{OAuthClient, OAuthError, OAuthProvider};

/// Builds a provider's OAuth client from the user's registration.
#[async_trait]
pub trait Credentials: Send + Sync {
    /// The client for `provider` (served by `oauth`); `NotConfigured` until
    /// the user has set it up.
    async fn client(
        &self,
        provider: CalendarProvider,
        oauth: &'static OAuthProvider,
    ) -> Result<OAuthClient, OAuthError>;
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
        oauth: &'static OAuthProvider,
    ) -> Result<Option<String>, OAuthError> {
        if !oauth.sends_client_secret {
            return Ok(None);
        }
        let secret = self
            .secrets
            .get(SecretName::GoogleOAuthClientSecret)
            .await
            .map_err(|error| {
                log::warn!(target: "oauth", "could not read the {} client secret: {error}", oauth.name);
                OAuthError::NotConfigured(oauth.name)
            })?
            .ok_or(OAuthError::NotConfigured(oauth.name))?;
        Ok(Some(secret.expose().to_owned()))
    }
}

#[async_trait]
impl Credentials for SettingsCredentials {
    async fn client(
        &self,
        provider: CalendarProvider,
        oauth: &'static OAuthProvider,
    ) -> Result<OAuthClient, OAuthError> {
        let calendar = self.settings.get().await.calendar;
        let connector = match provider {
            CalendarProvider::Google => &calendar.google,
            CalendarProvider::Microsoft => &calendar.microsoft,
            CalendarProvider::Apple => return Err(OAuthError::NotConfigured(oauth.name)),
        };
        let client_id = connector
            .client_id()
            .ok_or(OAuthError::NotConfigured(oauth.name))?
            .to_owned();
        let secret = self.client_secret(oauth).await?;
        Ok(OAuthClient::new(oauth, client_id, secret))
    }
}
