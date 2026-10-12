//! The recorder's system notifications: "recording started / stopped" on
//! the transitions, and a warning the first time each health issue opens
//! (CAP-10: a silent failure must not go unnoticed). The settings decide
//! whether each kind shows; meeting content is never put in a warning.

use deino_domain::Channel;
use deino_recording::{HealthIssue, IssueKind, RecordingPhase, RecordingSnapshot};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

use super::adapter::Follower;
use crate::notifications::commands::{
    NotificationManagerState, show_recording_started_notification,
    show_recording_stopped_notification,
};

/// Shows the recorder's notifications.
pub(super) struct Notifications;

/// A transition the user is notified of.
enum Transition {
    /// Recording started, of the meeting with this title.
    Started(Option<String>),
    /// Recording stopped.
    Stopped,
}

impl<R: Runtime> Follower<R> for Notifications {
    fn follow(&self, app: &AppHandle<R>, previous: &RecordingSnapshot, state: &RecordingSnapshot) {
        match (previous.phase, state.phase) {
            (RecordingPhase::Starting, RecordingPhase::Recording) => {
                notify(app, Transition::Started(state.title.clone()));
            }
            (RecordingPhase::Stopping, RecordingPhase::Idle) => notify(app, Transition::Stopped),
            _ => {}
        }
        let opened: Vec<HealthIssue> = state
            .issues
            .iter()
            .filter(|issue| !previous.issues.contains(issue))
            .copied()
            .collect();
        if !opened.is_empty() {
            warn(app, opened);
        }
    }
}

/// Shows the started or stopped notification, honouring the settings.
fn notify<R: Runtime>(app: &AppHandle<R>, transition: Transition) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(state) = app.try_state::<NotificationManagerState<R>>() else {
            return;
        };
        let shown = match transition {
            Transition::Started(title) => {
                show_recording_started_notification(&app, &state, title).await
            }
            Transition::Stopped => show_recording_stopped_notification(&app, &state).await,
        };
        if let Err(error) = shown {
            log::warn!(target: "recording", "could not show the recording notification: {error}");
        }
    });
}

/// Warns about newly opened health issues, when system-error notifications
/// are on.
fn warn<R: Runtime>(app: &AppHandle<R>, issues: Vec<HealthIssue>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let settings = crate::features::settings::current(&app).await;
        if !settings.notifications.preferences.show_system_errors {
            return;
        }
        for issue in issues {
            let shown = app
                .notification()
                .builder()
                .title(crate::APP_NAME)
                .body(warning(issue))
                .show();
            if let Err(error) = shown {
                log::warn!(target: "recording", "could not show a health warning: {error}");
            }
        }
    });
}

/// The warning for `issue`: what is wrong, and that recording goes on.
pub(crate) fn warning(issue: HealthIssue) -> &'static str {
    match (issue.kind, issue.channel) {
        (IssueKind::NoAudio, Some(Channel::System)) => {
            "No audio from the call for 5 seconds. Check that Deino may record system audio."
        }
        (IssueKind::NoAudio, _) => {
            "No microphone audio for 5 seconds. Check your microphone in the record panel."
        }
        (IssueKind::NoTranscript, _) => {
            "You are speaking but nothing is being transcribed. The audio is still recorded."
        }
        (IssueKind::TranscriptionStopped, _) => {
            "Transcription stopped and is restarting. The audio is still recorded."
        }
        (IssueKind::DiskLow, _) => "Your disk is nearly full (under 2 GB free).",
        (IssueKind::DiskFull, _) => "Your disk is full, so the recording was stopped and saved.",
    }
}
