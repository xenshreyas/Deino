//! Sessions: a connection's authorized client, token refresh and health.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::TimeDelta;
use deino_domain::{AppError, ConnectionId, ConnectionStatus, ErrorClass, IntegrationProvider};
use deino_net::{NetClient, NetError, Request, Response};
use deino_secrets::{SecretError, SecretValue};
use url::Url;

use super::IntegrationService;
use super::connect::oauth_error;
use crate::{
    Connection, ConnectionError, Credential, Integration, IntegrationError, OAuthError,
    ProviderApi, ProviderError, TokenEndpoint, TokenExtras, TokenScheme,
};

/// Refresh this long before the access token expires, so a request never
/// leaves with a token that dies on the way.
const EXPIRY_SKEW: TimeDelta = TimeDelta::minutes(2);

impl IntegrationService {
    /// Checks that a connection still works by asking the provider who it
    /// belongs to, and records the answer on the connection.
    pub async fn check(&self, id: ConnectionId) -> Result<Connection, IntegrationError> {
        let connection = self.connection(id).await?;
        let integration = Arc::clone(self.ports.registry.get(connection.provider)?);
        self.ensure_online(connection.provider).await?;
        let outcome = async {
            let api = self
                .session(&connection, integration.as_ref(), false)
                .await?;
            integration
                .identify(&api, &TokenExtras::default())
                .await
                .map_err(|error| self.provider_error(integration.as_ref(), error))
        }
        .await;
        match outcome {
            Ok(_) => {
                self.ports
                    .connections
                    .record_success(id, self.ports.clock.now())
                    .await?;
            }
            Err(error) => self.note_failure(&connection, &error).await,
        }
        self.connection(id).await
    }

    /// The authorized client of `connection`, refreshing its token first
    /// when it is about to expire, or when `force` (a 401 came back).
    pub(super) async fn session(
        &self,
        connection: &Connection,
        integration: &dyn Integration,
        force: bool,
    ) -> Result<ProviderApi, IntegrationError> {
        let provider = connection.provider;
        let credential = self
            .ports
            .vault
            .load(provider, connection.id)
            .await?
            .ok_or(IntegrationError::NeedsReconnect(provider))?;
        let credential =
            if (force || self.expiring(connection)) && credential.refresh_token().is_some() {
                let cause = Stale {
                    credential: &credential,
                    rejected: force,
                };
                self.refresh(connection, integration, cause).await?
            } else {
                credential
            };
        self.api_with(integration, &credential, connection.api_base_url.as_deref())
    }

    /// True when the connection's access token expires within [`EXPIRY_SKEW`].
    fn expiring(&self, connection: &Connection) -> bool {
        connection
            .token_expires_at
            .is_some_and(|at| at - EXPIRY_SKEW <= self.ports.clock.now())
    }

    /// Refreshes the connection's tokens, one refresh at a time; a caller
    /// that waited finds the tokens already renewed and uses them. The new
    /// pair is in the Keychain before anything uses it.
    async fn refresh(
        &self,
        connection: &Connection,
        integration: &dyn Integration,
        stale: Stale<'_>,
    ) -> Result<Credential, IntegrationError> {
        let provider = connection.provider;
        let _held = self.refreshing.hold(connection.id).await;
        let needs_reconnect = || IntegrationError::NeedsReconnect(provider);
        let current = self
            .ports
            .vault
            .load(provider, connection.id)
            .await?
            .ok_or_else(needs_reconnect)?;
        let latest = self.connection(connection.id).await?;
        let renewed_meanwhile = !same_access(&current, stale.credential)
            || (!stale.rejected && !self.expiring(&latest));
        if renewed_meanwhile {
            return Ok(current);
        }
        self.renew(connection, integration, &current).await
    }

    /// Exchanges the refresh token of `current` and stores the new pair.
    async fn renew(
        &self,
        connection: &Connection,
        integration: &dyn Integration,
        current: &Credential,
    ) -> Result<Credential, IntegrationError> {
        let provider = connection.provider;
        let needs_reconnect = || IntegrationError::NeedsReconnect(provider);
        let spec = integration
            .descriptor()
            .oauth()
            .ok_or_else(needs_reconnect)?;
        let refresh_token = current.refresh_token().ok_or_else(needs_reconnect)?;
        let (app, _) = self.oauth_client(provider, spec).await?;
        let client = self.client_for(provider);
        let endpoint = TokenEndpoint {
            client: client.as_ref(),
            url: &spec.token_url,
            auth: spec.client_auth,
            app: &app,
        };
        let tokens = match endpoint.refresh(refresh_token).await {
            Ok(tokens) => tokens,
            Err(OAuthError::InvalidGrant) => return Err(needs_reconnect()),
            Err(error) => return Err(oauth_error(provider, error)),
        };
        let kept = || SecretValue::parse_credential(refresh_token.expose()).ok();
        let renewed = Credential::OAuth {
            access: tokens.access_token,
            refresh: tokens.refresh_token.or_else(kept),
        };
        self.ports
            .vault
            .save(provider, connection.id, &renewed)
            .await?;
        let expires_at = self.expiry(tokens.expires_in);
        let now = self.ports.clock.now();
        self.ports
            .connections
            .set_token_expiry(connection.id, expires_at, now)
            .await?;
        log::debug!(target: "integrations", "refreshed connection {}", connection.id);
        Ok(renewed)
    }

    /// A [`ProviderApi`] that sends `credential` to the provider's service.
    pub(super) fn api_with(
        &self,
        integration: &dyn Integration,
        credential: &Credential,
        base_url: Option<&str>,
    ) -> Result<ProviderApi, IntegrationError> {
        let scheme = integration
            .descriptor()
            .token()
            .map_or(TokenScheme::Bearer, |spec| spec.scheme);
        let authorization = credential.authorization(scheme)?;
        let base_url = base_url.and_then(|url| Url::parse(url).ok());
        let client = self.client_for(integration.id());
        Ok(ProviderApi::new(client, Some(authorization)).with_base_url(base_url))
    }

    /// The client of `provider`'s network service; the sandbox has none.
    pub(super) fn client_for(&self, provider: IntegrationProvider) -> Arc<dyn NetClient> {
        match provider.service() {
            Some(service) => self.ports.egress.client(service),
            None => Arc::new(NoNetwork),
        }
    }

    /// What an adapter's `error` means, after the provider's classifier.
    pub(super) fn provider_error(
        &self,
        integration: &dyn Integration,
        error: ProviderError,
    ) -> IntegrationError {
        IntegrationError::from_provider(integration.id(), self.classified(integration, error))
    }

    /// `error`, with a raw HTTP failure classified by its provider.
    pub(super) fn classified(
        &self,
        integration: &dyn Integration,
        error: ProviderError,
    ) -> ProviderError {
        match error {
            ProviderError::Http(failure) => integration.classify(&failure, self.ports.clock.now()),
            other => other,
        }
    }

    /// Records `error` on the connection, unless it says nothing about it
    /// (offline mode, a switched-off service, a cancel).
    pub(super) async fn note_failure(&self, connection: &Connection, error: &IntegrationError) {
        let status = match error {
            IntegrationError::Offline(_)
            | IntegrationError::Disabled(_)
            | IntegrationError::Cancelled(_)
            | IntegrationError::InFlight => return,
            IntegrationError::NeedsReconnect(_)
            | IntegrationError::Secrets(SecretError::CredentialDenied) => {
                ConnectionStatus::NeedsReconnect
            }
            _ => ConnectionStatus::Connected,
        };
        let failure = ConnectionError::new(
            &AppError::new(error.code(), error.to_string()),
            self.ports.clock.now(),
        );
        let recorded = self
            .ports
            .connections
            .record_failure(connection.id, status, &failure)
            .await;
        if let Err(storage) = recorded {
            log::warn!(target: "integrations", "could not record a failure on {}: {storage}", connection.id);
        }
    }
}

/// Why a refresh is wanted: the credential a caller holds, and whether the
/// provider rejected it (a 401) or it is merely about to expire.
struct Stale<'a> {
    credential: &'a Credential,
    rejected: bool,
}

/// True when both credentials carry the same access token.
fn same_access(one: &Credential, other: &Credential) -> bool {
    match (one, other) {
        (Credential::OAuth { access: a, .. }, Credential::OAuth { access: b, .. }) => {
            a.expose() == b.expose()
        }
        _ => false,
    }
}

/// The client of a provider that sends nothing (the sandbox).
struct NoNetwork;

#[async_trait]
impl NetClient for NoNetwork {
    async fn send(&self, _request: Request) -> Result<Response, NetError> {
        Err(NetError::Connect(Box::new(std::io::Error::other(
            "this provider sends nothing",
        ))))
    }
}
