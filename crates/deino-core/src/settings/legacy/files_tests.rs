//! The JSON store import against files shaped like the legacy app wrote them.

use std::sync::Arc;

use super::*;
use crate::settings::{
    FakeSettingsStore, ModelDownloadState, RecordedSettingsEvents, Settings, SettingsService,
};

const RECORDING: &str = r#"{"preferences":{"save_folder":"/Users/me/Movies/Deino","auto_save":false,"file_format":"mp4","preferred_mic_device":"MacBook Pro Microphone (input)","preferred_system_device":null,"system_audio_backend":"coreaudio"}}"#;
const PREFERENCES: &str = r#"{"show_recording_notification":false}"#;
const ONBOARDING: &str = r#"{"status":{"version":"1.0","completed":true,"current_step":4,"model_status":{"parakeet":"downloaded","summary":"downloaded","selected_summary_model":"qwen3.5:2b"},"last_updated":"2026-05-30T00:00:00Z"}}"#;
const NOTIFICATIONS: &str = r#"{"recording_notifications":true,"time_based_reminders":false,"meeting_reminders":true,"respect_do_not_disturb":true,"notification_sound":false,"system_permission_granted":true,"consent_given":true,"manual_dnd_mode":false,"notification_preferences":{"show_recording_started":true,"show_recording_stopped":true,"show_recording_paused":true,"show_recording_resumed":true,"show_transcription_complete":true,"show_meeting_reminders":true,"show_system_errors":true,"meeting_reminder_minutes":[10]}}"#;

struct Harness {
    directory: tempfile::TempDir,
    settings: SettingsService,
}

async fn harness(files: &[(&str, &str)]) -> Harness {
    let directory = tempfile::tempdir().unwrap();
    for (name, contents) in files {
        std::fs::write(directory.path().join(name), contents).unwrap();
    }
    let store = Arc::new(FakeSettingsStore::default());
    let events = Arc::new(RecordedSettingsEvents::default());
    let settings = SettingsService::load(store, events).await.unwrap();
    Harness {
        directory,
        settings,
    }
}

impl Harness {
    async fn import(&self) -> Settings {
        let import = LegacyFileImport::new(self.directory.path());
        import.run(&self.settings).await.unwrap();
        self.settings.get().await
    }

    fn exists(&self, name: &str) -> bool {
        self.directory.path().join(name).exists()
    }
}

#[tokio::test]
async fn every_legacy_store_is_carried_over_then_deleted() {
    let harness = harness(&[
        ("recording_preferences.json", RECORDING),
        ("preferences.json", PREFERENCES),
        ("onboarding-status.json", ONBOARDING),
        ("notifications.json", NOTIFICATIONS),
        ("store.json", r#"{"serverAddress":"http://localhost:5167"}"#),
    ])
    .await;

    let settings = harness.import().await;

    assert!(!settings.recording.auto_save);
    assert!(!settings.recording.show_recording_notice);
    assert_eq!(
        settings.audio.preferred_mic_device.as_deref(),
        Some("MacBook Pro Microphone (input)")
    );
    assert!(settings.onboarding.completed && settings.onboarding.started());
    assert_eq!(settings.onboarding.summary, ModelDownloadState::Downloaded);
    assert!(settings.notifications.consent_given);
    assert_eq!(
        settings.notifications.preferences.meeting_reminder_minutes,
        [10]
    );
    for name in [
        "recording_preferences.json",
        "preferences.json",
        "onboarding-status.json",
        "notifications.json",
        "store.json",
    ] {
        assert!(!harness.exists(name), "{name} was not deleted");
    }
}

#[tokio::test]
async fn running_again_finds_nothing_and_changes_nothing() {
    let harness = harness(&[("preferences.json", PREFERENCES)]).await;
    let first = harness.import().await;

    let second = harness.import().await;

    assert_eq!(first, second);
}

#[tokio::test]
async fn an_unreadable_store_is_left_in_place_and_the_rest_still_moves() {
    let harness = harness(&[
        ("onboarding-status.json", "{ broken"),
        ("preferences.json", PREFERENCES),
    ])
    .await;

    let settings = harness.import().await;

    assert!(!settings.recording.show_recording_notice);
    assert!(!settings.onboarding.started());
    assert!(harness.exists("onboarding-status.json"));
}

#[tokio::test]
async fn a_legacy_value_that_breaks_a_rule_keeps_the_default() {
    let too_far = NOTIFICATIONS.replace("[10]", "[5000]");
    let harness = harness(&[("notifications.json", &too_far)]).await;

    let settings = harness.import().await;

    assert_eq!(settings.notifications, Settings::default().notifications);
    assert!(!harness.exists("notifications.json"));
}
