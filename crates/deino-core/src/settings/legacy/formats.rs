//! The shapes the legacy stores wrote, and how each becomes [`Settings`].
//!
//! Every field is optional or defaulted: the stores were written by several
//! app versions, and a missing field must keep today's default rather than
//! fail the import.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::super::{
    CustomOpenAiSettings, ModelDownloadState, NotificationSettings, OnboardingSettings, Settings,
};

/// The `customOpenAIConfig` JSON column of the legacy `settings` row.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct LegacyCustomOpenAi {
    endpoint: String,
    pub(super) api_key: Option<String>,
    model: String,
    max_tokens: Option<i64>,
    temperature: Option<f32>,
    top_p: Option<f32>,
}

impl LegacyCustomOpenAi {
    /// Parses the column; `None` (logged) when it is not the expected JSON.
    pub(super) fn parse(json: &str) -> Option<Self> {
        serde_json::from_str(json)
            .inspect_err(|error| {
                log::warn!(target: "settings", "skipped the unreadable legacy custom endpoint: {error}");
            })
            .ok()
    }

    /// The endpoint's settings, without its key (which goes to the Keychain).
    pub(super) fn into_settings(self) -> CustomOpenAiSettings {
        CustomOpenAiSettings {
            endpoint: self.endpoint.trim().to_owned(),
            model: self.model.trim().to_owned(),
            max_tokens: self
                .max_tokens
                .and_then(|tokens| u32::try_from(tokens).ok()),
            temperature: self.temperature,
            top_p: self.top_p,
        }
    }
}

/// `recording_preferences.json`: `{ "preferences": { … } }`.
#[derive(Debug, Deserialize)]
pub(super) struct RecordingFile {
    preferences: RecordingPreferences,
}

#[derive(Debug, Deserialize)]
struct RecordingPreferences {
    #[serde(default = "yes")]
    auto_save: bool,
    #[serde(default)]
    preferred_mic_device: Option<String>,
    #[serde(default)]
    preferred_system_device: Option<String>,
}

impl RecordingFile {
    pub(super) fn apply(self, settings: &mut Settings) {
        let preferences = self.preferences;
        settings.recording.auto_save = preferences.auto_save;
        settings.audio.preferred_mic_device = preferences.preferred_mic_device;
        settings.audio.preferred_system_device = preferences.preferred_system_device;
    }
}

/// `preferences.json`: the "tell participants" recording notice switch.
#[derive(Debug, Deserialize)]
pub(super) struct PreferencesFile {
    #[serde(default = "yes")]
    show_recording_notification: bool,
}

impl PreferencesFile {
    pub(super) fn apply(self, settings: &mut Settings) {
        settings.recording.show_recording_notice = self.show_recording_notification;
    }
}

/// `onboarding-status.json`: `{ "status": { … } }`.
#[derive(Debug, Deserialize)]
pub(super) struct OnboardingFile {
    status: OnboardingStatus,
}

#[derive(Debug, Deserialize)]
struct OnboardingStatus {
    #[serde(default)]
    completed: bool,
    #[serde(default = "first_step")]
    current_step: u8,
    #[serde(default)]
    model_status: ModelStatus,
    #[serde(default)]
    last_updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Deserialize)]
struct ModelStatus {
    #[serde(default)]
    parakeet: ModelDownloadState,
    #[serde(default)]
    summary: ModelDownloadState,
    #[serde(default)]
    selected_summary_model: Option<String>,
}

impl OnboardingFile {
    pub(super) fn apply(self, settings: &mut Settings) {
        let status = self.status;
        settings.onboarding = OnboardingSettings {
            completed: status.completed,
            current_step: status.current_step,
            parakeet: status.model_status.parakeet,
            summary: status.model_status.summary,
            selected_summary_model: status.model_status.selected_summary_model,
            // The file existing means onboarding saved progress.
            updated_at: Some(status.last_updated.unwrap_or(DateTime::UNIX_EPOCH)),
        };
    }
}

/// `notifications.json`: today's [`NotificationSettings`] under its legacy
/// `snake_case` names, which the section accepts as aliases.
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub(super) struct NotificationsFile(NotificationSettings);

impl NotificationsFile {
    pub(super) fn apply(self, settings: &mut Settings) {
        settings.notifications = self.0;
    }
}

const fn yes() -> bool {
    true
}

const fn first_step() -> u8 {
    1
}
