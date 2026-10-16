//! The `pipeline` plugin's typed event and the adapter that emits it.
//!
//! The runner reports every stage status change through its `ProgressSink`
//! port, and a long stage how far it has got; [`TauriPipelineProgress`]
//! implements the port by emitting [`PipelineProgress`]
//! (`plugin:pipeline:pipeline-progress`).

use deino_domain::{JobStatus, MeetingId, StageId};
use deino_pipeline::ProgressSink;
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// A stage of a meeting's pipeline changed status, or a running stage got
/// further.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct PipelineProgress {
    /// The meeting being processed.
    pub meeting_id: MeetingId,
    /// The stage whose status changed.
    pub stage: StageId,
    /// Its new status.
    pub status: JobStatus,
    /// How far the running stage is, as a whole percent from 0 to 100
    /// (rounded down), when it reports that (`final_transcription` does);
    /// null on a plain status change. Sent at most once per percent.
    pub percent: Option<u8>,
}

impl From<&deino_pipeline::PipelineProgress> for PipelineProgress {
    fn from(progress: &deino_pipeline::PipelineProgress) -> Self {
        Self {
            meeting_id: progress.meeting_id.clone(),
            stage: progress.stage.clone(),
            status: progress.status,
            percent: progress.percent(),
        }
    }
}

/// Emits pipeline progress to the UI.
pub struct TauriPipelineProgress<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriPipelineProgress<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> ProgressSink for TauriPipelineProgress<R> {
    fn progress(&self, progress: &deino_pipeline::PipelineProgress) {
        // A missed event only delays the strip; `jobs` is the truth and
        // `DbChanged` refreshes it as well.
        if let Err(error) = PipelineProgress::from(progress).emit(&self.app) {
            log::warn!(
                target: "pipeline",
                "could not emit pipeline-progress for meeting {}: {error}",
                progress.meeting_id
            );
        }
    }
}
