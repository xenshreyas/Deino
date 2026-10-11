//! Commands of the `recording` plugin: parse, call the recorder, map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::segments::Segment;
use deino_domain::{AppError, MeetingId};
use deino_recording::{RecordingService, RecordingSnapshot};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![start, stop, pause, resume, state, transcript]
}

/// Starts recording a new meeting titled `title` (a default when null or
/// blank) and returns its ID once audio is being captured. Fails with
/// `conflict` while a recording runs and `unavailable` when the microphone
/// or the transcription model cannot start.
#[tauri::command]
#[specta::specta]
async fn start(
    recording: State<'_, Arc<RecordingService>>,
    title: Option<String>,
) -> Result<MeetingId, AppError> {
    recording.start(title).await.into_ipc()
}

/// Stops the recording, saves its last lines and hands the meeting to the
/// post-meeting pipeline; returns its ID. Fails with `conflict` when
/// nothing is recording.
#[tauri::command]
#[specta::specta]
async fn stop(recording: State<'_, Arc<RecordingService>>) -> Result<MeetingId, AppError> {
    recording.stop().await.into_ipc()
}

/// Pauses capture; the recording skips the pause.
#[tauri::command]
#[specta::specta]
async fn pause(recording: State<'_, Arc<RecordingService>>) -> Result<RecordingSnapshot, AppError> {
    recording.pause().into_ipc()
}

/// Resumes a paused recording.
#[tauri::command]
#[specta::specta]
async fn resume(
    recording: State<'_, Arc<RecordingService>>,
) -> Result<RecordingSnapshot, AppError> {
    recording.resume().into_ipc()
}

/// The recorder's state now.
#[tauri::command]
#[specta::specta]
async fn state(recording: State<'_, Arc<RecordingService>>) -> Result<RecordingSnapshot, AppError> {
    Ok(recording.state())
}

/// The saved transcript of `meeting_id`, oldest first, each line on its
/// channel (Me = `mic`, Them = `system`). Live lines arrive as
/// `live-segment` events; saved ones also change `transcript_segments`.
#[tauri::command]
#[specta::specta]
async fn transcript(
    recording: State<'_, Arc<RecordingService>>,
    meeting_id: MeetingId,
) -> Result<Vec<Segment>, AppError> {
    recording.transcript(&meeting_id).await.into_ipc()
}
