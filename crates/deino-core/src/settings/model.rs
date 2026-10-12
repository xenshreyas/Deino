//! [`Settings`]: every user preference, as one typed, versioned document.

use serde::{Deserialize, Serialize};

use super::{
    AiSettings, AudioSettings, AutomationSettings, BetaSettings, BriefSettings, CalendarSettings,
    ConsentSettings, DetectionSettings, ExportSettings, IntegrationSettings, NetworkSettings,
    NotificationSettings, OnboardingSettings, OrganizeSettings, PrivacySettings, ProfileSettings,
    RecordingSettings, SettingsError, SpeakerSettings, TemplateSettings, TranscriptionSettings,
    UiSettings,
};

/// The settings format this build writes. Bump it, and add the upgrade to
/// [`Settings::upgraded`], when a change cannot be expressed by defaults.
pub const SETTINGS_VERSION: u32 = 1;

/// Every user preference. Secrets are never here; they live in the Keychain.
///
/// Each section, and each field in it, falls back to its default when the
/// file lacks it, so adding a field never breaks an existing file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// The format version; read-only for patches.
    pub version: u32,
    /// The LLM that writes notes.
    pub ai: AiSettings,
    /// Capture devices.
    pub audio: AudioSettings,
    /// Recording behaviour.
    pub recording: RecordingSettings,
    /// The local transcription engine.
    pub transcription: TranscriptionSettings,
    /// System notifications.
    pub notifications: NotificationSettings,
    /// Meeting detection and auto-start (CAP-06).
    pub detection: DetectionSettings,
    /// Naming speakers by voice (CAP-04).
    pub speakers: SpeakerSettings,
    /// The meeting-chat notice and the consent reminder (CAP-13).
    pub consent: ConsentSettings,
    /// Privacy controls (PRV-01).
    pub privacy: PrivacySettings,
    /// Which outside services may send data, and offline mode (FND-12).
    pub network: NetworkSettings,
    /// The user's profile (NOT-05).
    pub profile: ProfileSettings,
    /// Note templates (NOT-04).
    pub templates: TemplateSettings,
    /// Auto-tagging and auto-filing (ORG-05, ORG-06).
    pub organize: OrganizeSettings,
    /// Exports and auto-export (EXP-02).
    pub export: ExportSettings,
    /// Calendar sync and which calendars are shown (ORG-01).
    pub calendar: CalendarSettings,
    /// Pre-meeting briefs (NOT-08).
    pub briefs: BriefSettings,
    /// The user's own OAuth apps for integrations (INT-00).
    pub integrations: IntegrationSettings,
    /// What automations may do on the Mac (AUT-01).
    pub automations: AutomationSettings,
    /// Appearance and layout.
    pub ui: UiSettings,
    /// Opt-in beta features.
    pub beta: BetaSettings,
    /// First-run setup progress.
    pub onboarding: OnboardingSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            ai: AiSettings::default(),
            audio: AudioSettings::default(),
            recording: RecordingSettings::default(),
            transcription: TranscriptionSettings::default(),
            notifications: NotificationSettings::default(),
            detection: DetectionSettings::default(),
            speakers: SpeakerSettings::default(),
            consent: ConsentSettings::default(),
            privacy: PrivacySettings::default(),
            network: NetworkSettings::default(),
            profile: ProfileSettings::default(),
            templates: TemplateSettings::default(),
            organize: OrganizeSettings::default(),
            export: ExportSettings::default(),
            calendar: CalendarSettings::default(),
            automations: AutomationSettings::default(),
            briefs: BriefSettings::default(),
            integrations: IntegrationSettings::default(),
            ui: UiSettings::default(),
            beta: BetaSettings::default(),
            onboarding: OnboardingSettings::default(),
        }
    }
}

impl Settings {
    /// Checks every rule a section enforces.
    pub fn validate(&self) -> Result<(), SettingsError> {
        self.ai.validate()?;
        self.notifications.validate()?;
        self.profile.validate()?;
        self.detection.validate()?;
        self.export.validate()?;
        self.calendar.validate()?;
        self.speakers.validate()?;
        self.consent.validate()?;
        self.integrations.validate()?;
        self.privacy.validate()?;
        self.ui.validate()
    }

    /// These settings in the current format.
    ///
    /// Version 1 is the first format, so there is nothing to convert yet. A
    /// file from a newer build keeps the fields this build knows.
    #[must_use]
    pub fn upgraded(mut self) -> Self {
        if self.version > SETTINGS_VERSION {
            log::warn!(
                target: "settings",
                "settings file is version {}, newer than {SETTINGS_VERSION}; unknown fields are dropped",
                self.version
            );
        }
        self.version = SETTINGS_VERSION;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_file_is_the_defaults() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn a_partial_section_keeps_the_other_defaults() {
        let settings: Settings =
            serde_json::from_str(r#"{ "ui": { "theme": "dark" }, "future": 1 }"#).unwrap();

        assert_eq!(settings.ui.theme, super::super::Theme::Dark);
        assert!(settings.ui.show_confidence_indicator);
        assert!(settings.recording.auto_save);
    }

    #[test]
    fn defaults_are_valid_and_current() {
        let settings = Settings::default();
        assert!(settings.validate().is_ok());
        assert_eq!(settings.version, SETTINGS_VERSION);
    }

    #[test]
    fn upgrading_stamps_the_current_version() {
        let old = Settings {
            version: 0,
            ..Settings::default()
        };
        assert_eq!(old.upgraded().version, SETTINGS_VERSION);
    }
}
