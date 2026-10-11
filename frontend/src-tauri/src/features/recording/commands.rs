//! Commands of the `recording` plugin: parse, call the recorder, map the error.
//! The record panel's and auto-stop's commands are in [`controls`].

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

mod controls;

pub use controls::{MainWindow, Microphone, PanelRegion, TauriMainWindow};

use std::sync::Arc;

use deino_domain::{AppError, MeetingId};
use deino_recording::{RecordingService, RecordingSnapshot, StartRequest};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        start,
        stop,
        pause,
        resume,
        state,
        resume_meeting,
        controls::set_microphone,
        controls::set_system_audio,
        controls::microphones,
        controls::keep_recording,
        controls::answer_split,
        controls::auto_stop,
        controls::open_live_meeting,
        controls::set_panel_region,
    ]
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
    recording
        .start(StartRequest::titled(title))
        .await
        .into_ipc()
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

/// Resumes recording on the finished meeting `meeting_id` (CAP-07): new
/// audio and transcript are appended and the meeting is processed again
/// on stop. Fails with `conflict` while a recording runs or the meeting is
/// not finished, and `not_found` for an unknown meeting.
#[tauri::command]
#[specta::specta]
async fn resume_meeting(
    recording: State<'_, Arc<RecordingService>>,
    meeting_id: MeetingId,
) -> Result<MeetingId, AppError> {
    recording.resume_meeting(&meeting_id).await.into_ipc()
}
