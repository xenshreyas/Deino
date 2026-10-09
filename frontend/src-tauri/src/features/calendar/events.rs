//! The `calendar` plugin's typed event and the adapter that emits it.
//!
//! The core announces every status or visibility change through its
//! `CalendarEvents` port; [`TauriCalendarEvents`] emits it as
//! `plugin:calendar:calendar-changed`.

use deino_core::calendar::{CalendarEvents, CalendarStatus};
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// Calendar sync changed what the UI shows: the switch, macOS access, the
/// last sync, or which calendars are hidden. `status` is the new state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
pub struct CalendarChanged {
    /// The state of calendar sync now.
    pub status: CalendarStatus,
}

/// Emits calendar changes to the UI.
pub struct TauriCalendarEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriCalendarEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> CalendarEvents for TauriCalendarEvents<R> {
    fn changed(&self, status: &CalendarStatus) {
        let event = CalendarChanged {
            status: status.clone(),
        };
        // A window that misses it refetches the status when it next mounts.
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "calendar", "could not emit calendar-changed: {error}");
        }
    }
}
