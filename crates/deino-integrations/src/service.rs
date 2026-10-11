//! [`IntegrationService`]: the connection lifecycle and the delivery engine.
//!
//! - Listing: [`IntegrationService::providers`], [`IntegrationService::deliveries`].
//! - Connecting (`connect.rs`): OAuth in the browser or a pasted token; the
//!   account is identified, its credential goes to the Keychain, its row
//!   (no secrets) to `integration_connections`, and its network service is
//!   switched on. Disconnecting revokes, forgets, deletes and switches off.
//! - Sessions (`session.rs`): the authorized client of a connection,
//!   refreshing an expiring token first, one refresh at a time per
//!   connection (rotating refresh tokens die if reused).
//! - Delivering (`deliver.rs`): once per idempotency key, paced by the
//!   provider's rate limit, retried with backoff, logged.
//!
//! Offline mode stops all of it before anything is sent, with an error that
//! says so.

mod connect;
mod deliver;
mod disconnect;
mod pending;
mod reads;
mod session;

use std::sync::Arc;

use deino_domain::{Clock, ConnectionId, IntegrationProvider, ServiceId};

use self::pending::{PendingConnects, RefreshLocks};
use crate::catalog;
use crate::view::{planned_view, provider_view};
use crate::{
    AppInput, AppRegistration, Browser, Connection, ConnectionRepo, CredentialVault, Delivery,
    DeliveryRepo, Egress, IntegrationError, IntegrationRegistry, OAuthApps, ProviderView,
    RateLimiter, Sleeper,
};

/// The most delivery log rows one call returns.
pub const MAX_DELIVERY_PAGE: u32 = 200;

/// The longest client ID accepted; the settings enforce the same limit
/// (`deino_core::settings::MAX_CLIENT_ID_CHARS`), which this crate sits below.
const MAX_CLIENT_ID_CHARS: usize = 256;

/// Everything the service is built from.
pub struct IntegrationPorts {
    /// The providers this build ships.
    pub registry: IntegrationRegistry,
    /// Connected accounts.
    pub connections: Arc<dyn ConnectionRepo>,
    /// The delivery log.
    pub deliveries: Arc<dyn DeliveryRepo>,
    /// Credentials and client secrets in the Keychain.
    pub vault: CredentialVault,
    /// The user's app registrations.
    pub apps: Arc<dyn OAuthApps>,
    /// The network, its policy and its switches.
    pub egress: Arc<dyn Egress>,
    /// The browser for OAuth.
    pub browser: Arc<dyn Browser>,
    /// The time.
    pub clock: Arc<dyn Clock>,
    /// Waiting for rate limits and backoff.
    pub sleeper: Arc<dyn Sleeper>,
}

/// See the module docs.
pub struct IntegrationService {
    ports: IntegrationPorts,
    limiter: RateLimiter,
    pending: PendingConnects,
    refreshing: RefreshLocks,
}

impl IntegrationService {
    /// The service over its ports.
    pub fn new(ports: IntegrationPorts) -> Self {
        let limiter = RateLimiter::new(Arc::clone(&ports.clock), Arc::clone(&ports.sleeper));
        Self {
            ports,
            limiter,
            pending: PendingConnects::default(),
            refreshing: RefreshLocks::default(),
        }
    }

    /// Every provider this build ships, with the user's registrations and
    /// connections, then every planned provider it does not ship yet.
    pub async fn providers(&self) -> Result<Vec<ProviderView>, IntegrationError> {
        let connections = self.ports.connections.connections().await?;
        let mut views = Vec::new();
        for integration in self.ports.registry.iter() {
            let id = integration.id();
            let app = self.app(id).await?;
            let own = connections
                .iter()
                .filter(|c| c.provider == id)
                .cloned()
                .collect();
            views.push(provider_view(integration.as_ref(), app, own));
        }
        let planned = IntegrationProvider::ALL
            .iter()
            .copied()
            .filter(|&id| catalog::is_planned(id) && self.ports.registry.get(id).is_err());
        views.extend(planned.map(planned_view));
        Ok(views)
    }

    /// The newest `limit` deliveries (at most [`MAX_DELIVERY_PAGE`]).
    pub async fn deliveries(&self, limit: u32) -> Result<Vec<Delivery>, IntegrationError> {
        Ok(self
            .ports
            .deliveries
            .recent(limit.min(MAX_DELIVERY_PAGE))
            .await?)
    }

    /// Saves the user's app for `provider`: the client ID and port in the
    /// settings, a new client secret in the Keychain.
    pub async fn set_app(
        &self,
        provider: IntegrationProvider,
        input: AppInput,
    ) -> Result<ProviderView, IntegrationError> {
        let integration = self.ports.registry.get(provider)?;
        if integration.descriptor().oauth().is_none() {
            return Err(IntegrationError::MethodUnavailable(provider));
        }
        let client_id = input.client_id.trim();
        if client_id.is_empty() || client_id.chars().count() > MAX_CLIENT_ID_CHARS {
            return Err(IntegrationError::Invalid(
                "enter the app's client ID".into(),
            ));
        }
        if let Some(secret) = input
            .client_secret
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            let secret = deino_secrets::SecretValue::parse(secret)?;
            self.ports
                .vault
                .set_client_secret(provider, Some(secret))
                .await?;
        }
        let registration = AppRegistration {
            client_id: client_id.to_owned(),
            redirect_port: input.redirect_port.filter(|port| *port >= 1024),
        };
        self.ports.apps.save(provider, Some(registration)).await?;
        self.provider(provider).await
    }

    /// Forgets the user's app for `provider` and its client secret.
    pub async fn remove_app(
        &self,
        provider: IntegrationProvider,
    ) -> Result<ProviderView, IntegrationError> {
        self.ports.registry.get(provider)?;
        self.ports.vault.set_client_secret(provider, None).await?;
        self.ports.apps.save(provider, None).await?;
        self.provider(provider).await
    }

    /// The view of one provider.
    async fn provider(
        &self,
        provider: IntegrationProvider,
    ) -> Result<ProviderView, IntegrationError> {
        let integration = self.ports.registry.get(provider)?;
        let connections = self.ports.connections.connections().await?;
        let own = connections
            .into_iter()
            .filter(|c| c.provider == provider)
            .collect();
        let app = self.app(provider).await?;
        Ok(provider_view(integration.as_ref(), app, own))
    }

    /// The user's registration for `provider`, and whether a secret is saved.
    async fn app(
        &self,
        provider: IntegrationProvider,
    ) -> Result<Option<(AppRegistration, bool)>, IntegrationError> {
        let Some(registration) = self.ports.apps.registration(provider).await else {
            return Ok(None);
        };
        let has_secret = self.ports.vault.client_secret(provider).await?.is_some();
        Ok(Some((registration, has_secret)))
    }

    /// The connection `id`.
    async fn connection(&self, id: ConnectionId) -> Result<Connection, IntegrationError> {
        self.ports
            .connections
            .connection(id)
            .await?
            .ok_or(IntegrationError::ConnectionNotFound(id))
    }

    /// Switches `service` off unless another connection still uses it.
    async fn release_service(&self, service: ServiceId) -> Result<(), IntegrationError> {
        let connections = self.ports.connections.connections().await?;
        let in_use = connections
            .iter()
            .any(|c| c.provider.service() == Some(service));
        if !in_use {
            self.ports
                .egress
                .set_enabled(service, false)
                .await
                .map_err(|_| IntegrationError::Settings)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod oauth_tests;
#[cfg(test)]
mod service_tests;
#[cfg(test)]
mod test_harness;
