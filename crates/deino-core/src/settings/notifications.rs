//! The notifications section: which system notifications Deino shows.
//!
//! The `snake_case` aliases read the legacy `notifications.json` as is; the
//! settings file and the UI use the camelCase names.

use serde::{Deserialize, Serialize};

use super::SettingsError;

/// The longest reminder lead time, in minutes (a day).
pub const MAX_REMINDER_MINUTES: u16 = 1440;

/// The notifications section of `Settings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent on/off switches the user toggles one by one"
)]
pub struct NotificationSettings {
    /// Recording lifecycle notifications (start, stop, pause, resume).
    #[serde(alias = "recording_notifications")]
    pub recording_notifications: bool,
    /// Time-based meeting reminders.
    #[serde(alias = "time_based_reminders")]
    pub time_based_reminders: bool,
    /// Reminders for calendar events.
    #[serde(alias = "meeting_reminders")]
    pub meeting_reminders: bool,
    /// Stay quiet while the system's Do Not Disturb is on.
    #[serde(alias = "respect_do_not_disturb")]
    pub respect_do_not_disturb: bool,
    /// Play a sound with each notification.
    #[serde(alias = "notification_sound")]
    pub notification_sound: bool,
    /// macOS granted permission to show notifications.
    #[serde(alias = "system_permission_granted")]
    pub system_permission_granted: bool,
    /// The user agreed to receive notifications.
    #[serde(alias = "consent_given")]
    pub consent_given: bool,
    /// Deino's own Do Not Disturb switch.
    #[serde(alias = "manual_dnd_mode")]
    pub manual_dnd_mode: bool,
    /// Per-kind switches.
    #[serde(alias = "notification_preferences")]
    pub preferences: NotificationPreferences,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            recording_notifications: true,
            time_based_reminders: true,
            meeting_reminders: true,
            respect_do_not_disturb: true,
            notification_sound: true,
            system_permission_granted: false,
            consent_given: false,
            manual_dnd_mode: false,
            preferences: NotificationPreferences::default(),
        }
    }
}

impl NotificationSettings {
    /// Checks that every reminder fits in a day.
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        let minutes = &self.preferences.meeting_reminder_minutes;
        if minutes.iter().any(|lead| *lead > MAX_REMINDER_MINUTES) {
            return Err(SettingsError::invalid(
                "notifications.preferences.meetingReminderMinutes",
                "cannot be more than a day ahead",
            ));
        }
        Ok(())
    }
}

/// Which kinds of notification to show.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent on/off switches the user toggles one by one"
)]
pub struct NotificationPreferences {
    /// "Recording started".
    #[serde(alias = "show_recording_started")]
    pub show_recording_started: bool,
    /// "Recording stopped".
    #[serde(alias = "show_recording_stopped")]
    pub show_recording_stopped: bool,
    /// "Recording paused".
    #[serde(alias = "show_recording_paused")]
    pub show_recording_paused: bool,
    /// "Recording resumed".
    #[serde(alias = "show_recording_resumed")]
    pub show_recording_resumed: bool,
    /// "Transcription complete".
    #[serde(alias = "show_transcription_complete")]
    pub show_transcription_complete: bool,
    /// Meeting reminders.
    #[serde(alias = "show_meeting_reminders")]
    pub show_meeting_reminders: bool,
    /// System errors.
    #[serde(alias = "show_system_errors")]
    pub show_system_errors: bool,
    /// Minutes before a meeting to remind the user, each at most a day.
    #[serde(alias = "meeting_reminder_minutes")]
    pub meeting_reminder_minutes: Vec<u16>,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            show_recording_started: false,
            show_recording_stopped: false,
            show_recording_paused: true,
            show_recording_resumed: true,
            show_transcription_complete: true,
            show_meeting_reminders: true,
            show_system_errors: true,
            meeting_reminder_minutes: vec![15, 5],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminders_more_than_a_day_ahead_are_rejected() {
        let mut settings = NotificationSettings::default();
        settings.preferences.meeting_reminder_minutes = vec![MAX_REMINDER_MINUTES + 1];

        assert!(settings.validate().is_err());
    }
}
