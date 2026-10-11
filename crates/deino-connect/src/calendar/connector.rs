//! [`ConnectorSource`]: a provider's calendar API behind the core's
//! `CalendarSource`, signed in with OAuth.
//!
//! Connecting switches the provider's network service on (FND-12: an
//! integration sends nothing until the user connects it), signs in through
//! the browser and keeps the refresh token in the Keychain; the access
//! token lives in memory only. Every read refreshes an expired access token
//! first, and retries once after a 401. Disconnecting forgets the token and
//! switches the service off again.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::calendar::{
    CalendarAccess, CalendarSource, SourceError, SourceSnapshot, SyncWindow,
};
use deino_domain::{CalendarProvider, Clock};
use deino_net::{NetClient, NetError, ServiceSwitch, Status};
use deino_secrets::{SecretStore, SecretValue};
use tokio::sync::Mutex;

use super::api::CalendarApi;
use super::credentials::Credentials;
use crate::oauth::{BrowserOpener, Grant, OAuthError, SignIn, TokenSet, sign_in};

/// What a connector is built from.
pub struct ConnectorParts {
    /// The provider's calendar API.
    pub api: Arc<dyn CalendarApi>,
    /// A client scoped to the provider's service.
    pub http: Arc<dyn NetClient>,
    /// The user's app registration.
    pub credentials: Arc<dyn Credentials>,
    /// Where the refresh token is kept.
    pub secrets: Arc<dyn SecretStore>,
    /// Switches the provider's network service on and off.
    pub switch: Arc<dyn ServiceSwitch>,
    /// Shows the sign-in page.
    pub browser: Arc<dyn BrowserOpener>,
    /// Stamps token expiry.
    pub clock: Arc<dyn Clock>,
}

/// A calendar provider reached directly over its API.
pub struct ConnectorSource {
    parts: ConnectorParts,
    access: Mutex<Option<TokenSet>>,
}

impl ConnectorSource {
    /// A connector over `parts`; not connected until signed in.
    pub fn new(parts: ConnectorParts) -> Self {
        Self {
            parts,
            access: Mutex::new(None),
        }
    }

    /// The saved refresh token, if the account is connected.
    async fn refresh_token(&self) -> Result<Option<String>, SourceError> {
        let secret = self.parts.api.refresh_token_secret();
        let value = self.parts.secrets.get(secret).await.map_err(failed)?;
        Ok(value.map(|value| value.expose().to_owned()))
    }

    /// Keeps a (new) refresh token.
    async fn save_refresh_token(&self, token: &str) -> Result<(), SourceError> {
        let value = SecretValue::parse(token).map_err(failed)?;
        let secret = self.parts.api.refresh_token_secret();
        self.parts.secrets.set(secret, value).await.map_err(failed)
    }

    /// A current access token: the cached one, or a refreshed one.
    async fn access_token(&self) -> Result<String, SourceError> {
        let mut cached = self.access.lock().await;
        let now = self.parts.clock.now();
        if let Some(tokens) = cached.as_ref().filter(|tokens| !tokens.is_stale(now)) {
            return Ok(tokens.access_token.clone());
        }
        let refresh = self
            .refresh_token()
            .await?
            .ok_or(SourceError::AccessDenied)?;
        let client = self.client().await?;
        let tokens = client
            .exchange(self.parts.http.as_ref(), Grant::Refresh(&refresh), now)
            .await
            .map_err(source_error)?;
        if let Some(rotated) = tokens.refresh_token.as_deref() {
            self.save_refresh_token(rotated).await?;
        }
        let access = tokens.access_token.clone();
        *cached = Some(tokens);
        Ok(access)
    }

    async fn client(&self) -> Result<crate::oauth::OAuthClient, SourceError> {
        let api = &self.parts.api;
        self.parts
            .credentials
            .client(api.provider(), api.oauth())
            .await
            .map_err(source_error)
    }

    /// One read of the API with a current token.
    async fn read(&self, window: SyncWindow) -> Result<SourceSnapshot, NetError> {
        let token = self
            .access_token()
            .await
            .map_err(|_| NetError::Status(Status::new(401)))?;
        self.parts
            .api
            .snapshot(self.parts.http.as_ref(), &token, window)
            .await
    }

    /// Switches the provider's network service on or off.
    async fn set_service(&self, enabled: bool) -> Result<(), SourceError> {
        let service = self.parts.api.service();
        self.parts
            .switch
            .set_enabled(service, enabled)
            .await
            .map_err(failed)
    }
}

#[async_trait]
impl CalendarSource for ConnectorSource {
    fn provider(&self) -> CalendarProvider {
        self.parts.api.provider()
    }

    async fn access(&self) -> CalendarAccess {
        match self.refresh_token().await {
            Ok(Some(_)) => CalendarAccess::Granted,
            Ok(None) | Err(_) => CalendarAccess::NotDetermined,
        }
    }

    async fn request_access(&self) -> Result<CalendarAccess, SourceError> {
        let client = self.client().await?;
        self.set_service(true).await?;
        let flow = SignIn {
            client: &client,
            http: self.parts.http.as_ref(),
            browser: self.parts.browser.as_ref(),
            clock: self.parts.clock.as_ref(),
        };
        let tokens = sign_in(flow).await.map_err(source_error)?;
        let refresh = tokens
            .refresh_token
            .clone()
            .ok_or_else(|| source_error(OAuthError::InvalidResponse(client.provider().name)))?;
        self.save_refresh_token(&refresh).await?;
        *self.access.lock().await = Some(tokens);
        Ok(CalendarAccess::Granted)
    }

    async fn fetch(&self, window: SyncWindow) -> Result<SourceSnapshot, SourceError> {
        if self.refresh_token().await?.is_none() {
            return Err(SourceError::AccessDenied);
        }
        match self.read(window).await {
            Err(NetError::Status(status)) if status.as_u16() == 401 => {
                // The access token died early (revoked session): refresh once.
                *self.access.lock().await = None;
                self.read(window).await.map_err(net_error)
            }
            result => result.map_err(net_error),
        }
    }

    async fn disconnect(&self) -> Result<(), SourceError> {
        *self.access.lock().await = None;
        let secret = self.parts.api.refresh_token_secret();
        self.parts.secrets.delete(secret).await.map_err(failed)?;
        self.set_service(false).await
    }
}

/// An OAuth failure as a source error: a refused sign-in is lost access.
fn source_error(error: OAuthError) -> SourceError {
    match error {
        OAuthError::Revoked(_) | OAuthError::Denied { .. } => SourceError::AccessDenied,
        other => failed(other),
    }
}

/// A request failure as a source error.
fn net_error(error: NetError) -> SourceError {
    match error {
        NetError::Status(status) if status.as_u16() == 401 => SourceError::AccessDenied,
        other => failed(other),
    }
}

fn failed(error: impl std::error::Error + Send + Sync + 'static) -> SourceError {
    SourceError::Failed(Box::new(error))
}

#[cfg(test)]
mod connector_tests;
