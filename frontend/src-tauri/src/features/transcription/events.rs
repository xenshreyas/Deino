//! Typed events of the `transcription` plugin and the model-event adapter.
//!
//! Names are namespaced by the plugin, for example
//! `plugin:transcription:live-transcript`.

use deino_core::models::{DownloadProgress, ModelEvents, ModelFileId};
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// A model download made progress.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct ModelDownloadProgress {
    /// Bytes on disk so far.
    pub progress: DownloadProgress,
}

/// A model file's state changed: started, finished, failed, cancelled or deleted.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct ModelChanged {
    /// The file.
    pub id: ModelFileId,
}

/// Emits the model service's events to the UI.
pub struct TauriModelEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriModelEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> ModelEvents for TauriModelEvents<R> {
    fn progress(&self, progress: &DownloadProgress) {
        let event = ModelDownloadProgress {
            progress: progress.clone(),
        };
        // A missed progress tick is harmless; the next one carries the total.
        if let Err(error) = event.emit(&self.app) {
            log::debug!(target: "models", "could not emit download progress: {error}");
        }
    }

    fn changed(&self, id: &ModelFileId) {
        let event = ModelChanged { id: id.clone() };
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "models", "could not emit model-changed for {id}: {error}");
        }
    }
}
