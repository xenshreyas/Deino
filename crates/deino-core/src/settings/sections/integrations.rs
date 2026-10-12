//! The integrations section: the user's own OAuth app per provider (INT-00).
//!
//! Deino ships no client IDs and no secrets (PARITY_PLAN §4.5). A provider
//! that connects through OAuth uses the app the user registered with it: the
//! client ID and, for providers that match it exactly, the redirect port are
//! here; a client secret, when the provider needs one, is in the Keychain
//! (`integration.<provider>.client-secret`), never in this file. The
//! Integrations page writes this section through the `integrations` plugin.

use std::collections::BTreeMap;

use deino_domain::IntegrationProvider;
use serde::{Deserialize, Serialize};

use crate::settings::{MAX_CLIENT_ID_CHARS, SettingsError};

/// The lowest redirect port a user app may register: unprivileged ports only.
pub const MIN_REDIRECT_PORT: u16 = 1024;

/// The integrations section of `Settings`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct IntegrationSettings {
    /// The user's OAuth app per provider.
    pub apps: BTreeMap<IntegrationProvider, OAuthAppSettings>,
}

/// One OAuth app the user registered with a provider.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct OAuthAppSettings {
    /// The app's client ID.
    pub client_id: String,
    /// The redirect port the app registered, for providers that match it
    /// exactly (Slack, Linear); `None` uses the provider's default.
    pub redirect_port: Option<u16>,
}

impl IntegrationSettings {
    /// Every app has a client ID of sensible length and an unprivileged port.
    pub fn validate(&self) -> Result<(), SettingsError> {
        for app in self.apps.values() {
            let id = app.client_id.trim();
            if id.is_empty() || id.chars().count() > MAX_CLIENT_ID_CHARS {
                return Err(SettingsError::invalid(
                    "integrations.apps.clientId",
                    "must be 1 to 256 characters",
                ));
            }
            if app
                .redirect_port
                .is_some_and(|port| port < MIN_REDIRECT_PORT)
            {
                return Err(SettingsError::invalid(
                    "integrations.apps.redirectPort",
                    "must be 1024 or higher",
                ));
            }
        }
        Ok(())
    }
}

crate::settings::register_section! {
    /// The user's own OAuth apps for integrations (INT-00).
    integrations: IntegrationSettings,
    order: 180,
    validate: IntegrationSettings::validate,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(client_id: &str, redirect_port: Option<u16>) -> IntegrationSettings {
        let app = OAuthAppSettings {
            client_id: client_id.into(),
            redirect_port,
        };
        IntegrationSettings {
            apps: BTreeMap::from([(IntegrationProvider::Linear, app)]),
        }
    }

    #[test]
    fn no_apps_are_configured_by_default() {
        assert!(IntegrationSettings::default().apps.is_empty());
    }

    #[test]
    fn apps_are_keyed_by_the_stored_provider_name() {
        let settings: IntegrationSettings = serde_json::from_str(
            r#"{ "apps": { "linear": { "clientId": "abc", "redirectPort": 53682 } } }"#,
        )
        .unwrap();

        assert_eq!(settings, app("abc", Some(53_682)));
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn a_blank_client_id_and_a_privileged_port_are_refused() {
        assert!(app(" ", None).validate().is_err());
        assert!(app("abc", Some(80)).validate().is_err());
    }
}
