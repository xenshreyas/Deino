//! The local REST API section (DEV-03): whether Deino serves its API on
//! `127.0.0.1`, and on which port.

use serde::{Deserialize, Serialize};

use crate::settings::SettingsError;

/// The port the API listens on unless the user picks another.
pub const DEFAULT_API_PORT: u16 = 47_810;
/// The lowest port the API may use: unprivileged ports only.
pub const MIN_API_PORT: u16 = 1024;

/// The API section of `Settings`. Changed only through the `api` plugin,
/// which starts and stops the server to match (a plain patch is refused).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ApiSettings {
    /// Serve the API. Off by default.
    pub enabled: bool,
    /// The loopback port it listens on.
    pub port: u16,
}

impl Default for ApiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_API_PORT,
        }
    }
}

impl ApiSettings {
    /// Checks the port is unprivileged.
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.port < MIN_API_PORT {
            return Err(SettingsError::invalid("api.port", "must be 1024 or higher"));
        }
        Ok(())
    }
}

crate::settings::register_section! {
    /// The local REST API (DEV-03).
    api: ApiSettings,
    order: 230,
    validate: ApiSettings::validate,
    dialog_only: ["enabled", "port"],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_api_is_off_on_its_default_port_unless_the_file_says_otherwise() {
        let missing: ApiSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(missing, ApiSettings::default());
        assert!(!missing.enabled);
        assert_eq!(missing.port, DEFAULT_API_PORT);
    }

    #[test]
    fn a_privileged_port_is_refused() {
        let low = ApiSettings {
            enabled: true,
            port: 80,
        };
        assert!(low.validate().is_err());
        assert!(ApiSettings::default().validate().is_ok());
    }
}
