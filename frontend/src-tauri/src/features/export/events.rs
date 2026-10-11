//! The `export` plugin's typed event and the adapter that emits it.

use deino_core::export::{FullExportEvents, FullExportStatus};
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// The account export's status changed (`plugin:export:full-export-progress`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct FullExportProgress {
    /// The new status.
    pub status: FullExportStatus,
}

/// Emits account export progress to the UI.
pub struct TauriFullExportEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriFullExportEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> FullExportEvents for TauriFullExportEvents<R> {
    fn changed(&self, status: &FullExportStatus) {
        let event = FullExportProgress {
            status: status.clone(),
        };
        // A missed event only delays the progress bar; `full_export_status`
        // has the truth.
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "export", "could not emit full-export-progress: {error}");
        }
    }
}
