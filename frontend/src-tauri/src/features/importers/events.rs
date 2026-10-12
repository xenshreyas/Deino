//! The `importers` plugin's progress event and the adapter that emits it:
//! `plugin:importers:import-progress-changed`.

use deino_core::importers::{ImportEvents, ImportProgress};
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// The import job moved on.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct ImportProgressChanged {
    /// Where it is now.
    pub progress: ImportProgress,
}

/// Emits the job's progress to the UI.
pub struct TauriImportEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriImportEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> ImportEvents for TauriImportEvents<R> {
    fn progress(&self, progress: &ImportProgress) {
        let event = ImportProgressChanged {
            progress: progress.clone(),
        };
        // A missed event only delays the bar; `status` still answers.
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "importers", "could not emit the import progress: {error}");
        }
    }
}
