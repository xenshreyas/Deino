//! [`TauriRecordingEvents`]: the recorder's events port over Tauri. It
//! emits the typed events and tells its [`Follower`]s about each state
//! change: the menu bar, the started/stopped/health notifications and the
//! record panel follow the recorder this way.

use std::sync::{Mutex, PoisonError};

use deino_recording::{HealthNotice, RecordingEvents, RecordingSnapshot, SegmentUpdate};
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

use super::events::{Health, LiveSegment, RecordingState};
use super::notify;

/// Something that follows the recorder's state. Called on the thread that
/// changed it, under the recorder's state lock: implementations return
/// quickly and never call back into the recorder synchronously.
pub trait Follower<R: Runtime>: Send + Sync {
    /// The state went from `previous` to `state`.
    fn follow(&self, app: &AppHandle<R>, previous: &RecordingSnapshot, state: &RecordingSnapshot);
}

/// Emits the recorder's events to the UI.
pub struct TauriRecordingEvents<R: Runtime> {
    app: AppHandle<R>,
    last: Mutex<RecordingSnapshot>,
    followers: Vec<Box<dyn Follower<R>>>,
}

impl<R: Runtime> TauriRecordingEvents<R> {
    /// An adapter that emits through `app`, keeps the menu bar in step and
    /// shows the recording notifications.
    pub fn new(app: AppHandle<R>) -> Self {
        Self {
            app,
            last: Mutex::new(RecordingSnapshot::idle()),
            followers: vec![Box::new(MenuBar), Box::new(notify::Notifications)],
        }
    }

    /// Adds `follower` (the record panel, in the real app).
    #[must_use]
    pub fn with(mut self, follower: impl Follower<R> + 'static) -> Self {
        self.followers.push(Box::new(follower));
        self
    }
}

/// Refreshes the menu bar when the phase changes.
struct MenuBar;

impl<R: Runtime> Follower<R> for MenuBar {
    fn follow(&self, app: &AppHandle<R>, previous: &RecordingSnapshot, state: &RecordingSnapshot) {
        if previous.phase != state.phase || previous.calendar_event_id != state.calendar_event_id {
            crate::tray::refresh(app);
        }
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
        let previous = {
            let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
            std::mem::replace(&mut *last, state.clone())
        };
        for follower in &self.followers {
            follower.follow(&self.app, &previous, state);
        }
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
