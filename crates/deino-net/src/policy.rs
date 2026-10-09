//! The policy ports: what the user allows, read before every request.
//!
//! `deino-net` does not know where the user's choices live. The app
//! implements [`ServicePolicy`] and [`ServiceSwitch`] over the typed settings
//! (`network.offlineMode`, `network.services`, and the custom LLM endpoints),
//! so a switch flipped in Settings applies to the very next request.

use std::error::Error;

use async_trait::async_trait;
use deino_domain::ServiceId;

use crate::HostPattern;

/// What the user allows one service to do right now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    /// The service may send: the user's switch, or its registry default.
    pub enabled: bool,
    /// Offline mode is on: nothing may be sent except approved downloads.
    pub offline_mode: bool,
    /// Hosts the user's settings add (a custom endpoint), on top of the
    /// registry's.
    pub custom_hosts: Vec<HostPattern>,
}

/// Reads the user's network choices.
#[async_trait]
pub trait ServicePolicy: Send + Sync {
    /// What `service` may do now.
    async fn grant(&self, service: ServiceId) -> Grant;
}

/// A switch that could not be saved; the source says why (for the log).
#[derive(Debug, thiserror::Error)]
#[error("could not save the network settings")]
pub struct SwitchError(#[source] pub Box<dyn Error + Send + Sync>);

/// Changes the user's network choices.
#[async_trait]
pub trait ServiceSwitch: Send + Sync {
    /// Switches `service` on or off.
    async fn set_enabled(&self, service: ServiceId, enabled: bool) -> Result<(), SwitchError>;
}

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeServicePolicy;

#[cfg(any(test, feature = "test-support"))]
mod fake {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use async_trait::async_trait;
    use deino_domain::ServiceId;

    use super::{Grant, ServicePolicy, ServiceSwitch, SwitchError};
    use crate::{HostPattern, registry};

    /// The user's choices in memory: registry defaults until a test changes them.
    #[derive(Debug, Default)]
    pub struct FakeServicePolicy {
        state: Mutex<State>,
    }

    #[derive(Debug, Default)]
    struct State {
        offline_mode: bool,
        switches: BTreeMap<ServiceId, bool>,
        custom_hosts: BTreeMap<ServiceId, BTreeSet<String>>,
    }

    impl FakeServicePolicy {
        /// Turns offline mode on or off.
        pub fn set_offline(&self, offline: bool) {
            self.lock().offline_mode = offline;
        }

        /// Adds a custom host to `service`, like a configured endpoint.
        pub fn add_custom_host(&self, service: ServiceId, host: &str) {
            let mut state = self.lock();
            state
                .custom_hosts
                .entry(service)
                .or_default()
                .insert(host.to_owned());
        }

        fn lock(&self) -> MutexGuard<'_, State> {
            self.state.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    #[async_trait]
    impl ServicePolicy for FakeServicePolicy {
        async fn grant(&self, service: ServiceId) -> Grant {
            let state = self.lock();
            let default = registry::spec(service).enabled_by_default;
            let hosts = state.custom_hosts.get(&service).into_iter().flatten();
            Grant {
                enabled: state.switches.get(&service).copied().unwrap_or(default),
                offline_mode: state.offline_mode,
                custom_hosts: hosts.map(|host| HostPattern::exact(host)).collect(),
            }
        }
    }

    #[async_trait]
    impl ServiceSwitch for FakeServicePolicy {
        async fn set_enabled(&self, service: ServiceId, enabled: bool) -> Result<(), SwitchError> {
            self.lock().switches.insert(service, enabled);
            Ok(())
        }
    }
}
