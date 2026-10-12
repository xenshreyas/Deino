//! The privacy section.
//!
//! NOT-09 adds "Hide sensitive information"; PRV-02 adds encryption at rest
//! and the app lock; PRV-01 adds retention and the record panel's pause
//! tip. Each field is `#[serde(default)]`, so existing files keep loading.

use serde::{Deserialize, Serialize};

use crate::settings::{RetentionSettings, SettingsError};

/// The shortest idle time before the app locks, in minutes.
pub const MIN_LOCK_IDLE_MINUTES: u16 = 1;
/// The longest idle time before the app locks, in minutes (eight hours).
pub const MAX_LOCK_IDLE_MINUTES: u16 = 480;
/// The idle time a new install locks after, in minutes.
pub const DEFAULT_LOCK_IDLE_MINUTES: u16 = 5;

/// The privacy section of `Settings`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PrivacySettings {
    /// Replace passwords, API keys, card numbers, SSNs and one-time codes
    /// with `[redacted]` before notes are written and in exports (NOT-09).
    /// The transcript keeps the originals. Off by default.
    pub redact_sensitive: bool,
    /// Keep the database encrypted with a key in the Keychain (PRV-02,
    /// ADR 0041). The file is converted at the next launch, before it opens.
    /// Off by default.
    pub encrypt_database: bool,
    /// Lock the window behind Touch ID or the Mac's password (PRV-02).
    pub app_lock: AppLockSettings,
    /// How long recordings, transcripts and meetings are kept (PRV-01).
    pub retention: RetentionSettings,
    /// The record panel has shown "Pause for sensitive discussions" and the
    /// user dismissed it (PRV-01); it is shown once.
    pub pause_tip_seen: bool,
}

impl PrivacySettings {
    /// Checks the app lock's idle time and the retention periods.
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        self.app_lock.validate()?;
        self.retention.validate()
    }
}

/// When the app locks itself (PRV-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AppLockSettings {
    /// Lock at all. Off by default.
    pub enabled: bool,
    /// Lock after this many minutes without using Deino.
    pub idle_minutes: u16,
    /// Lock when the Mac wakes from sleep or its screen locks.
    pub lock_on_wake: bool,
}

impl Default for AppLockSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            idle_minutes: DEFAULT_LOCK_IDLE_MINUTES,
            lock_on_wake: true,
        }
    }
}

impl AppLockSettings {
    /// Checks that the idle time is within its allowed range.
    fn validate(self) -> Result<(), SettingsError> {
        let range = MIN_LOCK_IDLE_MINUTES..=MAX_LOCK_IDLE_MINUTES;
        if !range.contains(&self.idle_minutes) {
            return Err(SettingsError::invalid(
                "privacy.appLock.idleMinutes",
                "is outside its allowed range",
            ));
        }
        Ok(())
    }
}

crate::settings::register_section! {
    /// Privacy controls (PRV-01).
    privacy: PrivacySettings,
    order: 90,
    validate: PrivacySettings::validate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_is_off_by_default_and_in_old_files() {
        let old: PrivacySettings = serde_json::from_str("{}").unwrap();

        assert!(!old.redact_sensitive);
        assert!(!old.encrypt_database);
        assert!(!old.app_lock.enabled);
        assert_eq!(old.app_lock.idle_minutes, DEFAULT_LOCK_IDLE_MINUTES);
        assert_eq!(old, PrivacySettings::default());
    }

    #[test]
    fn the_fields_are_camel_case() {
        let on = PrivacySettings {
            redact_sensitive: true,
            encrypt_database: true,
            app_lock: AppLockSettings {
                enabled: true,
                idle_minutes: 10,
                lock_on_wake: false,
            },
            retention: RetentionSettings::default(),
            pause_tip_seen: true,
        };

        assert_eq!(
            serde_json::to_string(&on).unwrap(),
            r#"{"redactSensitive":true,"encryptDatabase":true,"appLock":{"enabled":true,"idleMinutes":10,"lockOnWake":false},"retention":{"audio":"never","transcript":"never","everything":"never"},"pauseTipSeen":true}"#
        );
    }

    #[test]
    fn an_idle_time_outside_the_range_is_rejected() {
        let mut privacy = PrivacySettings::default();
        for minutes in [0, MAX_LOCK_IDLE_MINUTES + 1] {
            privacy.app_lock.idle_minutes = minutes;
            assert!(privacy.validate().is_err(), "{minutes} minutes");
        }
        privacy.app_lock.idle_minutes = MAX_LOCK_IDLE_MINUTES;
        assert!(privacy.validate().is_ok());
    }
}
