//! The privacy section.
//!
//! NOT-09 adds "Hide sensitive information"; PRV-01/PRV-02 add retention
//! and app lock. Each field is `#[serde(default)]`, so existing files keep
//! loading.

use serde::{Deserialize, Serialize};

/// The privacy section of `Settings`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PrivacySettings {
    /// Replace passwords, API keys, card numbers, SSNs and one-time codes
    /// with `[redacted]` before notes are written and in exports (NOT-09).
    /// The transcript keeps the originals. Off by default.
    pub redact_sensitive: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_is_off_by_default_and_in_old_files() {
        let old: PrivacySettings = serde_json::from_str("{}").unwrap();

        assert!(!old.redact_sensitive);
        assert_eq!(old, PrivacySettings::default());
    }

    #[test]
    fn the_field_is_camel_case() {
        let on = PrivacySettings {
            redact_sensitive: true,
        };

        assert_eq!(
            serde_json::to_string(&on).unwrap(),
            r#"{"redactSensitive":true}"#
        );
    }
}
