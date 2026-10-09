//! The `notes` plugin's typed event and the adapter that emits it
//! (`plugin:notes:notes-saved`).

use deino_core::notes::{NotesEvents, SavedNotes};
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// A meeting's notes were saved.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct NotesSaved {
    /// Which meeting, and when.
    pub saved: SavedNotes,
}

/// Emits notes events to every window.
pub struct TauriNotesEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriNotesEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> NotesEvents for TauriNotesEvents<R> {
    fn saved(&self, saved: &SavedNotes) {
        let event = NotesSaved {
            saved: saved.clone(),
        };
        // The notes are saved either way; a window that misses this refetches
        // on `DbChanged`.
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "notes", "could not emit notes-saved for {}: {error}", saved.meeting_id);
        }
    }
}
