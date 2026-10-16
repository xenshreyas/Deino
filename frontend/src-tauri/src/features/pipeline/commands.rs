//! Commands of the `pipeline` plugin: read a meeting's stages, re-run them.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_domain::{AppError, MeetingId, StageId};
use deino_pipeline::{PipelineService, RequestedRepass, StageStatus};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The after-meeting transcription stage, where a re-transcription starts.
const TRANSCRIPT_STAGE: &str = "final_transcription";

/// The plugin's commands, for its `tauri_specta::Builder`.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![status, rerun, retranscribe]
}

/// The meeting's pipeline stages in the order they run, with their status.
/// Empty when the pipeline never ran for the meeting.
#[tauri::command]
#[specta::specta]
async fn status(
    pipeline: State<'_, Arc<PipelineService>>,
    meeting_id: MeetingId,
) -> Result<Vec<StageStatus>, AppError> {
    pipeline.status(&meeting_id).await.into_ipc()
}

/// Runs `stage` of the meeting again, or every stage when `stage` is null,
/// and returns the meeting's stages. Fails with `not_found` for an unknown
/// meeting, `invalid` for an unknown stage and `conflict` while recording.
#[tauri::command]
#[specta::specta]
async fn rerun(
    pipeline: State<'_, Arc<PipelineService>>,
    meeting_id: MeetingId,
    stage: Option<StageId>,
) -> Result<Vec<StageStatus>, AppError> {
    pipeline.rerun(&meeting_id, stage.as_ref()).await.into_ipc()
}

/// Transcribes the meeting again with the engine in Settings (echo removed,
/// edits overwritten), then runs every later step on the new transcript;
/// returns the meeting's stages. `conflict` while it is recording.
#[tauri::command]
#[specta::specta]
async fn retranscribe(
    pipeline: State<'_, Arc<PipelineService>>,
    requests: State<'_, Arc<RequestedRepass>>,
    meeting_id: MeetingId,
) -> Result<Vec<StageStatus>, AppError> {
    requests.request(&meeting_id);
    let stage = StageId::new(TRANSCRIPT_STAGE);
    pipeline.rerun_from(&meeting_id, &stage).await.into_ipc()
}
