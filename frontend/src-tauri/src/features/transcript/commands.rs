//! Commands of the `transcript` plugin: parse, call the service, map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::segments::Segment;
use deino_core::transcript::{
    ReplaceOutcome, ReplacePreview, ReplaceRequest, Transcript, TranscriptEditService,
    TranscriptService,
};
use deino_domain::{AppError, MeetingId, SegmentId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![get, edit, delete, restore, preview_replace, replace]
}

/// The saved transcript of `meeting_id`: its segments in spoken order,
/// each with its speaker ID (the participants name them). An unknown meeting
/// has an empty transcript.
#[tauri::command]
#[specta::specta]
async fn get(
    transcript: State<'_, Arc<TranscriptService>>,
    meeting_id: MeetingId,
) -> Result<Transcript, AppError> {
    transcript.transcript(&meeting_id).await.into_ipc()
}

/// Sets a line's text (trimmed, not blank) and returns the line; the first
/// edit keeps what the engine heard in `originalText`.
#[tauri::command]
#[specta::specta]
async fn edit(
    edits: State<'_, Arc<TranscriptEditService>>,
    segment_id: SegmentId,
    text: String,
) -> Result<Segment, AppError> {
    edits.edit(&segment_id, &text).await.into_ipc()
}

/// Deletes a line: it leaves the transcript, the notes' input, search and
/// exports, and can be restored.
#[tauri::command]
#[specta::specta]
async fn delete(
    edits: State<'_, Arc<TranscriptEditService>>,
    segment_id: SegmentId,
) -> Result<(), AppError> {
    edits.delete(&segment_id).await.into_ipc()
}

/// Brings a deleted line back (the delete toast's Undo).
#[tauri::command]
#[specta::specta]
async fn restore(
    edits: State<'_, Arc<TranscriptEditService>>,
    segment_id: SegmentId,
) -> Result<(), AppError> {
    edits.restore(&segment_id).await.into_ipc()
}

/// What replacing `request.find` would change: matches, lines and meetings.
#[tauri::command]
#[specta::specta]
async fn preview_replace(
    edits: State<'_, Arc<TranscriptEditService>>,
    request: ReplaceRequest,
) -> Result<ReplacePreview, AppError> {
    edits.preview(&request).await.into_ipc()
}

/// Replaces every match of `request.find` with `replacement`, in one
/// transaction.
#[tauri::command]
#[specta::specta]
async fn replace(
    edits: State<'_, Arc<TranscriptEditService>>,
    request: ReplaceRequest,
    replacement: String,
) -> Result<ReplaceOutcome, AppError> {
    edits.replace(&request, &replacement).await.into_ipc()
}
