//! Commands of the `screen` plugin: call one service method, map the
//! error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::screen::{ScreenService, Snapshot};
use deino_domain::{AppError, MeetingId};
use deino_screen::{CaptureStatus, ScreenCapture};
use serde::Serialize;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        status,
        open_screen_recording_settings,
        turn_off_for_meeting,
        snapshots
    ]
}

/// Screen capture on this Mac, for Settings and the record panel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(super) struct ScreenStatus {
    /// Whether this platform can capture the screen at all.
    supported: bool,
    /// Whether Screen Recording is granted (read without prompting).
    permitted: bool,
    /// What the capture of the current recording is doing.
    capture: CaptureStatus,
}

/// Whether Deino can capture, and what the capture is doing now.
#[tauri::command]
#[specta::specta]
async fn status(capture: State<'_, Arc<ScreenCapture>>) -> Result<ScreenStatus, AppError> {
    Ok(ScreenStatus {
        supported: capture.supported(),
        permitted: capture.permitted(),
        capture: capture.status(),
    })
}

/// Opens System Settings → Privacy & Security → Screen Recording. Fails
/// with `unavailable` where there is no such page (off macOS).
#[tauri::command]
#[specta::specta]
async fn open_screen_recording_settings() -> Result<(), AppError> {
    deino_screen::open_screen_recording_settings().into_ipc()
}

/// Stops the meeting's screen capture and keeps it off; true when a
/// capture stopped.
#[tauri::command]
#[specta::specta]
async fn turn_off_for_meeting(
    screen: State<'_, Arc<ScreenService>>,
    meeting_id: MeetingId,
) -> Result<bool, AppError> {
    screen.turn_off(&meeting_id).await.into_ipc()
}

/// The meeting's kept snapshots, in time order.
#[tauri::command]
#[specta::specta]
async fn snapshots(
    screen: State<'_, Arc<ScreenService>>,
    meeting_id: MeetingId,
) -> Result<Vec<Snapshot>, AppError> {
    screen.snapshots(&meeting_id).await.into_ipc()
}
