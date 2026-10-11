//! The `recording` plugin's typed events and the adapter that emits them.
//!
//! Names are namespaced by the plugin: `plugin:recording:recording-state`,
//! `plugin:recording:live-segment`, `plugin:recording:health`. The adapter
//! also keeps the tray menu in step with the phase and shows the "recording
//! started / stopped" notifications on the transitions.

use std::sync::{Mutex, PoisonError};

use deino_recording::{
    HealthNotice, RecordingEvents, RecordingPhase, RecordingSnapshot, SegmentUpdate,
};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use crate::notifications::commands::{
    NotificationManagerState, show_recording_started_notification,
    show_recording_stopped_notification,
};

/// The recorder's state changed (phase, meeting, pause time, echo mode).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct RecordingState {
    /// The new state.
    pub snapshot: RecordingSnapshot,
}

/// A live transcript line grew, closed (saved) or was discarded as echo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct LiveSegment {
    /// The line, on its channel (Me = `mic`, Them = `system`).
    pub update: SegmentUpdate,
}

/// Something about the recording's audio the user may need to know.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    /// What happened.
    pub notice: HealthNotice,
}

/// A transition the user is notified of.
enum Transition {
    /// Recording started, of the meeting with this title.
    Started(Option<String>),
    /// Recording stopped.
    Stopped,
}

/// Emits the recorder's events to the UI.
pub struct TauriRecordingEvents<R: Runtime> {
    app: AppHandle<R>,
    phase: Mutex<RecordingPhase>,
}

impl<R: Runtime> TauriRecordingEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self {
            app,
            phase: Mutex::new(RecordingPhase::Idle),
        }
    }

    /// Refreshes the tray and notifies on the start and stop transitions.
    fn follow(&self, state: &RecordingSnapshot) {
        let previous = {
            let mut phase = self.phase.lock().unwrap_or_else(PoisonError::into_inner);
            std::mem::replace(&mut *phase, state.phase)
        };
        if previous == state.phase {
            return;
        }
        crate::tray::update_tray_menu(&self.app);
        match (previous, state.phase) {
            (RecordingPhase::Starting, RecordingPhase::Recording) => {
                self.notify(Transition::Started(state.title.clone()));
            }
            (RecordingPhase::Stopping, RecordingPhase::Idle) => self.notify(Transition::Stopped),
            _ => {}
        }
    }

    /// Shows the started or stopped notification, honouring the user's
    /// notification settings.
    fn notify(&self, transition: Transition) {
        let app = self.app.clone();
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
}

impl<R: Runtime> RecordingEvents for TauriRecordingEvents<R> {
    fn state(&self, state: &RecordingSnapshot) {
        let event = RecordingState {
            snapshot: state.clone(),
        };
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "recording", "could not emit recording-state: {error}");
        }
        self.follow(state);
    }

    fn segment(&self, segment: &SegmentUpdate) {
        let event = LiveSegment {
            update: segment.clone(),
        };
        // A missed update is replaced by the next one, or by the saved row.
        if let Err(error) = event.emit(&self.app) {
            log::debug!(target: "recording", "could not emit live-segment: {error}");
        }
    }

    fn health(&self, notice: &HealthNotice) {
        let event = Health {
            notice: notice.clone(),
        };
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "recording", "could not emit health: {error}");
        }
    }
}
