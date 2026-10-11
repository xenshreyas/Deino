//! Typed events of the `search` plugin and the adapters that emit them.
//!
//! Names are namespaced by the plugin, for example
//! `plugin:search:index-progress`.

use deino_core::models::{DownloadProgress, ModelEvents, ModelFileId};
use deino_core::search::semantic::{BackfillProgress, IndexEvents};
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// The semantic backfill moved on.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct IndexProgress {
    /// Where it is.
    pub progress: BackfillProgress,
}

/// The embedding model's download made progress.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct EmbeddingModelProgress {
    /// Bytes on disk so far.
    pub progress: DownloadProgress,
}

/// The embedding model's file changed: download started, finished, failed,
/// was cancelled or deleted.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct EmbeddingModelChanged {
    /// The file.
    pub id: ModelFileId,
}

/// Emits the backfill's progress to the UI.
pub struct TauriIndexEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriIndexEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> IndexEvents for TauriIndexEvents<R> {
    fn backfill(&self, progress: &BackfillProgress) {
        let event = IndexProgress {
            progress: *progress,
        };
        // A missed tick is harmless: the next one, or a status read, catches up.
        if let Err(error) = event.emit(&self.app) {
            log::debug!(target: "search", "could not emit index progress: {error}");
        }
    }
}

/// Emits the embedding model service's events to the UI.
pub struct TauriEmbeddingModelEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriEmbeddingModelEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> ModelEvents for TauriEmbeddingModelEvents<R> {
    fn progress(&self, progress: &DownloadProgress) {
        let event = EmbeddingModelProgress {
            progress: progress.clone(),
        };
        if let Err(error) = event.emit(&self.app) {
            log::debug!(target: "search", "could not emit model progress: {error}");
        }
    }

    fn changed(&self, id: &ModelFileId) {
        let event = EmbeddingModelChanged { id: id.clone() };
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "search", "could not emit model-changed for {id}: {error}");
        }
    }
}
