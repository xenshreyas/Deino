//! The ports the integrations service needs from the app: the browser, the
//! network policy, the user's OAuth app registrations and time.
//!
//! The app's composition root implements them (the system URL opener,
//! `deino-net`'s `Net` and settings policy, `Settings::integrations`, the
//! tokio clock); tests use the fakes in `fake.rs`.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::{Clock, IntegrationProvider, ServiceId};
use deino_net::{Net, NetClient, ServicePolicy, ServiceSwitch, SwitchError};
use url::Url;

use crate::{IntegrationError, OAuthError};

/// Opens a URL in the user's default browser.
#[async_trait]
pub trait Browser: Send + Sync {
    /// Opens `url`; fails with [`OAuthError::Browser`] when it cannot.
    async fn open(&self, url: &Url) -> Result<(), OAuthError>;
}

/// The network as integrations see it: one client per service, offline
/// mode, and the service switch a connection turns on (FND-12).
#[async_trait]
pub trait Egress: Send + Sync {
    /// A client confined to `service`.
    fn client(&self, service: ServiceId) -> Arc<dyn NetClient>;

    /// True while offline mode is on.
    async fn offline(&self) -> bool;

    /// Switches `service` on (a connection was made) or off (its last
    /// connection was removed).
    async fn set_enabled(&self, service: ServiceId, enabled: bool) -> Result<(), SwitchError>;
}

/// [`Egress`] over `deino-net`: scoped clients, the settings policy and its
/// switch.
pub struct NetEgress {
    net: Net,
    policy: Arc<dyn ServicePolicy>,
    switch: Arc<dyn ServiceSwitch>,
}

impl NetEgress {
    /// Egress over `net`, reading and changing the user's choices through
    /// `policy` and `switch`.
    pub fn new(net: Net, policy: Arc<dyn ServicePolicy>, switch: Arc<dyn ServiceSwitch>) -> Self {
        Self {
            net,
            policy,
            switch,
        }
    }
}

#[async_trait]
impl Egress for NetEgress {
    fn client(&self, service: ServiceId) -> Arc<dyn NetClient> {
        Arc::new(self.net.scoped(service))
    }

    async fn offline(&self) -> bool {
        // Any integration service reports the same global switch.
        self.policy.grant(ServiceId::Slack).await.offline_mode
    }

    async fn set_enabled(&self, service: ServiceId, enabled: bool) -> Result<(), SwitchError> {
        self.switch.set_enabled(service, enabled).await
    }
}

/// The user's own registration with a provider: what Settings → Integrations
/// stores next to the provider. The client secret, if any, is not here: it
/// lives in the Keychain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppRegistration {
    /// The client ID.
    pub client_id: String,
    /// The redirect port the app registered, replacing the provider's default.
    pub redirect_port: Option<u16>,
}

/// Where the user's registrations are stored (`Settings::integrations`).
#[async_trait]
pub trait OAuthApps: Send + Sync {
    /// The registration for `provider`, if the user entered one.
    async fn registration(&self, provider: IntegrationProvider) -> Option<AppRegistration>;

    /// Saves (or, with `None`, forgets) the registration for `provider`;
    /// fails with [`IntegrationError::Settings`].
    async fn save(
        &self,
        provider: IntegrationProvider,
        registration: Option<AppRegistration>,
    ) -> Result<(), IntegrationError>;
}

/// Waits until a deadline on the injected clock. The same contract as the
/// pipeline's `Sleeper` (`deino-pipeline`, which sits above this crate).
#[async_trait]
pub trait Sleeper: Send + Sync {
    /// Returns once the clock reads `deadline` or later.
    async fn sleep_until(&self, deadline: DateTime<Utc>);
}

/// The real [`Sleeper`]: tokio timers, measured against the injected clock.
#[derive(Clone, Debug)]
pub struct ClockSleeper {
    clock: Arc<dyn Clock>,
}

impl ClockSleeper {
    /// A sleeper whose deadlines are read from `clock`.
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self { clock }
    }
}

#[async_trait]
impl Sleeper for ClockSleeper {
    async fn sleep_until(&self, deadline: DateTime<Utc>) {
        while let Ok(remaining) = (deadline - self.clock.now()).to_std() {
            if remaining.is_zero() {
                return;
            }
            tokio::time::sleep(remaining).await;
        }
    }
}
