//! Typed events of the `transcription` plugin and the model-event adapter.
//!
//! Names are namespaced by the plugin, for example
//! `plugin:transcription:live-transcript`.

use deino_core::models::{DownloadProgress, ModelEvents, ModelFileId};
use deino_stt::Lane;
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// Which audio a live line comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum LiveLane {
    /// The user's microphone.
    Microphone,
    /// System audio (the other side of the call).
    System,
    /// Both mixed, until CAP-02 splits the lanes.
    Mixed,
}

impl From<Lane> for LiveLane {
    fn from(lane: Lane) -> Self {
        match lane {
            Lane::Microphone => Self::Microphone,
            Lane::System => Self::System,
            Lane::Mixed => Self::Mixed,
        }
    }
}

/// The text of the segment being spoken now, as it grows. Empty text means
/// the segment closed (it arrives as a final `transcript-update`).
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct LiveTranscript {
    /// The lane it comes from.
    pub lane: LiveLane,
    /// Committed text: final, never rewritten.
    pub committed: String,
    /// The uncommitted tail, which may still change (empty for Parakeet).
    pub tentative: String,
    /// When the segment started, in seconds from the recording start.
    pub start_seconds: f64,
}

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
