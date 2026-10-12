//! The network section: which outside services may send data (FND-12).
//!
//! `deino-net` reads it through its `ServicePolicy` port before every
//! request. A service the user never switched has no entry in
//! [`NetworkSettings::services`] and keeps the default `deino-net` gives it,
//! so adding a service never needs a settings migration.

use std::collections::BTreeMap;

use deino_domain::ServiceId;
use serde::{Deserialize, Serialize};

/// The network section of `Settings`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct NetworkSettings {
    /// Send nothing at all, except model downloads the user starts. Notes
    /// then fall back to the built-in local model. Off by default.
    pub offline_mode: bool,
    /// The user's own on/off switch per service. A service without an entry
    /// uses its default (on for the LLM providers and model downloads, which
    /// only send when the user sets them up; off for everything else).
    pub services: BTreeMap<ServiceId, bool>,
}

impl NetworkSettings {
    /// The user's switch for `service`, if they ever flipped it.
    pub fn choice(&self, service: ServiceId) -> Option<bool> {
        self.services.get(&service).copied()
    }
}

crate::settings::register_section! {
    /// Which outside services may send data, and offline mode (FND-12).
    network: NetworkSettings,
    order: 100,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_mode_is_off_and_no_service_is_switched_by_default() {
        let network = NetworkSettings::default();
        assert!(!network.offline_mode);
        assert_eq!(network.choice(ServiceId::LlmAnthropic), None);
    }

    #[test]
    fn switches_are_keyed_by_the_stored_service_name() {
        let network: NetworkSettings =
            serde_json::from_str(r#"{ "offlineMode": true, "services": { "slack": true } }"#)
                .unwrap();

        assert!(network.offline_mode);
        assert_eq!(network.choice(ServiceId::Slack), Some(true));
    }
}
