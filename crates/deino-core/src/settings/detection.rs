//! The detection section: meeting detection and auto-start (CAP-06).
//!
//! `deino-detect` reads it through the app's adapter on every evaluation, so
//! a change applies within one detection tick, without a restart.

use std::collections::HashSet;

use deino_domain::BundleId;
use serde::{Deserialize, Serialize};

use super::SettingsError;

/// The shortest auto-start countdown, in seconds: time to read the prompt.
pub const MIN_AUTO_START_SECONDS: u16 = 3;

/// The longest auto-start countdown, in seconds: the prompt closes at 30 s.
pub const MAX_AUTO_START_SECONDS: u16 = 30;

/// The default countdown once auto-start is switched on (Circleback's
/// "short countdown").
const DEFAULT_AUTO_START_SECONDS: u16 = 10;

/// The detection section of `Settings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct DetectionSettings {
    /// Prompt to take notes when another app starts using the microphone.
    pub enabled: bool,
    /// Apps that never prompt ("Always ignore this app"), in the order added.
    pub ignored_apps: Vec<IgnoredApp>,
    /// Start recording on its own after a countdown, instead of waiting for
    /// "Take notes".
    pub auto_start: AutoStartSettings,
}

impl Default for DetectionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            ignored_apps: Vec::new(),
            auto_start: AutoStartSettings::default(),
        }
    }
}

/// An app the user chose to always ignore.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredApp {
    /// The app's bundle identifier, which detection matches.
    pub bundle_id: BundleId,
    /// The app's name when it was ignored, shown in Settings.
    pub name: String,
}

/// "Auto-start after N-second countdown" (off by default).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AutoStartSettings {
    /// Whether a detected meeting starts recording after the countdown.
    pub enabled: bool,
    /// The countdown, in seconds, during which "Don't record" cancels it.
    pub countdown_seconds: u16,
}

impl Default for AutoStartSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            countdown_seconds: DEFAULT_AUTO_START_SECONDS,
        }
    }
}

impl DetectionSettings {
    /// Whether `bundle_id` is on the ignore list.
    pub fn ignores(&self, bundle_id: &BundleId) -> bool {
        self.ignored_apps
            .iter()
            .any(|app| &app.bundle_id == bundle_id)
    }

    /// Adds `app` to the ignore list, unless it is already there.
    pub fn ignore(&mut self, app: IgnoredApp) {
        if !self.ignores(&app.bundle_id) {
            self.ignored_apps.push(app);
        }
    }

    /// Checks the countdown range and that no app is listed twice.
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        let seconds = self.auto_start.countdown_seconds;
        if !(MIN_AUTO_START_SECONDS..=MAX_AUTO_START_SECONDS).contains(&seconds) {
            return Err(SettingsError::invalid(
                "detection.autoStart.countdownSeconds",
                "must be between 3 and 30 seconds",
            ));
        }
        let mut seen = HashSet::new();
        if !self
            .ignored_apps
            .iter()
            .all(|app| seen.insert(&app.bundle_id))
        {
            return Err(SettingsError::invalid(
                "detection.ignoredApps",
                "lists an app more than once",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str) -> IgnoredApp {
        IgnoredApp {
            bundle_id: BundleId::try_from(id).unwrap(),
            name: id.to_owned(),
        }
    }

    #[test]
    fn detection_is_on_and_auto_start_off_by_default() {
        let settings = DetectionSettings::default();

        assert!(settings.enabled);
        assert!(!settings.auto_start.enabled);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn ignoring_an_app_twice_keeps_one_entry() {
        let mut settings = DetectionSettings::default();
        settings.ignore(app("us.zoom.xos"));
        settings.ignore(app("us.zoom.xos"));

        assert_eq!(settings.ignored_apps.len(), 1);
        assert!(settings.ignores(&BundleId::try_from("us.zoom.xos").unwrap()));
    }

    #[test]
    fn countdowns_outside_three_to_thirty_seconds_are_rejected() {
        for seconds in [0, 2, 31] {
            let mut settings = DetectionSettings::default();
            settings.auto_start.countdown_seconds = seconds;
            assert!(settings.validate().is_err(), "{seconds}");
        }
    }

    #[test]
    fn a_duplicated_ignore_entry_is_rejected() {
        let settings = DetectionSettings {
            ignored_apps: vec![app("us.zoom.xos"), app("us.zoom.xos")],
            ..DetectionSettings::default()
        };

        assert!(settings.validate().is_err());
    }

    #[test]
    fn camel_case_json_round_trips() {
        let json = r#"{"enabled":false,"ignoredApps":[{"bundleId":"us.zoom.xos","name":"Zoom"}],"autoStart":{"enabled":true,"countdownSeconds":5}}"#;

        let settings: DetectionSettings = serde_json::from_str(json).unwrap();

        assert_eq!(serde_json::to_string(&settings).unwrap(), json);
    }
}
