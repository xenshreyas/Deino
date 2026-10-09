//! Typed events of the `example` plugin and the adapter that emits them.
//!
//! The core announces domain moments through its `ExampleEvents` port;
//! [`TauriExampleEvents`] implements that port by emitting [`NoteAdded`].
//! Event names are namespaced by the plugin: `plugin:example:note-added`.

use deino_core::_example::{ExampleEvents, ExampleNote};
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// A note was saved.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct NoteAdded {
    /// The saved note.
    pub note: ExampleNote,
}

/// Emits the example's domain events to the UI.
pub struct TauriExampleEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriExampleEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> ExampleEvents for TauriExampleEvents<R> {
    fn note_added(&self, note: &ExampleNote) {
        let event = NoteAdded { note: note.clone() };
        // An event the UI misses only costs a highlight; the save already succeeded.
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "example", "could not emit note-added for {}: {error}", note.id);
        }
    }
}
