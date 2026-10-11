//! [`ConnectorSource`]: a provider's calendar API behind the core's
//! `CalendarSource`, signed in with OAuth.
//!
//! The OAuth protocol is the integrations framework's (`deino_integrations::oauth`,
//! ADR 0031): PKCE over a loopback redirect, the token endpoint and the
//! browser port. Connecting switches the provider's network service on
//! (FND-12: an integration sends nothing until the user connects it), signs
//! in through the browser and keeps the refresh token in the Keychain; the
//! access token lives in memory only. Every read refreshes an expired access
//! token first (one refresh at a time: the cache lock is held across it, so
//! a rotated refresh token is never reused), and retries once after a 401.
//! Disconnecting forgets the token and switches the service off again.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use deino_core::calendar::{
    CalendarAccess, CalendarSource, SourceError, SourceSnapshot, SyncWindow,
};
use deino_domain::{CalendarProvider, Clock};
use deino_integrations::oauth::AuthorizationFlow;
use deino_integrations::{Browser, IssuedTokens, OAuthClient, OAuthError, TokenEndpoint};
use deino_net::{NetClient, NetError, ServiceSwitch, Status};
use deino_secrets::{SecretStore, SecretValue};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use super::api::CalendarApi;
use super::credentials::Credentials;

/// An access token is refreshed this long before it expires, so a request
/// never leaves with one that dies in flight.
const EXPIRY_MARGIN: TimeDelta = TimeDelta::seconds(60);

/// How long an access token lives when the provider does not say.
const DEFAULT_LIFETIME: TimeDelta = TimeDelta::hours(1);

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
    pub browser: Arc<dyn Browser>,
    /// Stamps token expiry.
    pub clock: Arc<dyn Clock>,
}

/// An access token in memory, and when it stops working.
struct Access {
    token: SecretValue,
    expires_at: DateTime<Utc>,
}

/// A calendar provider reached directly over its API.
pub struct ConnectorSource {
    parts: ConnectorParts,
    access: Mutex<Option<Access>>,
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
    async fn refresh_token(&self) -> Result<Option<SecretValue>, SourceError> {
        let secret = self.parts.api.refresh_token_secret();
        self.parts.secrets.get(secret).await.map_err(failed)
    }

    /// Keeps a (new) refresh token, before the access token beside it is used.
    async fn save_refresh_token(&self, token: SecretValue) -> Result<(), SourceError> {
        let secret = self.parts.api.refresh_token_secret();
        self.parts.secrets.set(secret, token).await.map_err(failed)
    }

    /// A current access token: the cached one, or a refreshed one. The lock
    /// is held across the refresh, so refreshes never overlap.
    async fn access_token(&self) -> Result<String, SourceError> {
        let mut cached = self.access.lock().await;
        let now = self.parts.clock.now();
        if let Some(access) = cached
            .as_ref()
            .filter(|access| now + EXPIRY_MARGIN < access.expires_at)
        {
            return Ok(access.token.expose().to_owned());
        }
        let refresh = self
            .refresh_token()
            .await?
            .ok_or(SourceError::AccessDenied)?;
        let client = self.client().await?;
        let spec = self.parts.api.oauth();
        let endpoint = TokenEndpoint {
            client: self.parts.http.as_ref(),
            url: &spec.token_url,
            auth: spec.client_auth,
            app: &client,
        };
        let tokens = endpoint.refresh(&refresh).await.map_err(source_error)?;
        let access = self.keep(tokens).await?;
        let token = access.token.expose().to_owned();
        *cached = Some(access);
        Ok(token)
    }

    /// Saves a rotated refresh token and returns the access token to cache.
    async fn keep(&self, tokens: IssuedTokens) -> Result<Access, SourceError> {
        if let Some(rotated) = tokens.refresh_token {
            self.save_refresh_token(rotated).await?;
        }
        let lifetime = tokens
            .expires_in
            .and_then(|lifetime| TimeDelta::from_std(lifetime).ok())
            .unwrap_or(DEFAULT_LIFETIME);
        Ok(Access {
            token: tokens.access_token,
            expires_at: self.parts.clock.now() + lifetime,
        })
    }

    async fn client(&self) -> Result<OAuthClient, SourceError> {
        let api = &self.parts.api;
        self.parts
            .credentials
            .client(api.provider(), &api.oauth())
            .await
            .map_err(failed)
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

    /// Signs in in the browser and returns the tokens.
    async fn sign_in(&self, client: &OAuthClient) -> Result<IssuedTokens, SourceError> {
        let spec = self.parts.api.oauth();
        let flow = AuthorizationFlow {
            spec: &spec,
            app: client,
            client: self.parts.http.as_ref(),
            browser: self.parts.browser.as_ref(),
            port: None,
        };
        // The calendar page has no cancel button: the flow ends with the
        // browser's answer or its 5-minute limit.
        let tokens = flow
            .run(&CancellationToken::new())
            .await
            .map_err(source_error)?;
        tokens.ok_or(SourceError::AccessDenied)
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
        let tokens = self.sign_in(&client).await?;
        if tokens.refresh_token.is_none() {
            log::warn!(target: "oauth", "{} sent no refresh token", self.parts.api.provider());
            return Err(SourceError::AccessDenied);
        }
        let access = self.keep(tokens).await?;
        *self.access.lock().await = Some(access);
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

/// An OAuth failure as a source error: a refused sign-in or refresh is lost
/// access.
fn source_error(error: OAuthError) -> SourceError {
    match error {
        OAuthError::InvalidGrant | OAuthError::AccessDenied | OAuthError::Provider(_) => {
            SourceError::AccessDenied
        }
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
