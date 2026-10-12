//! Commands of the `screen` plugin: call one service method, map the
//! error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::screen::{ConsentReminders, ScreenService, Snapshot};
use deino_domain::{AppError, MeetingId};
use deino_screen::{CaptureStatus, ScreenCapture, Source, TargetChoice};
use serde::Serialize;
use tauri::{Runtime, State};

use super::ScreenPicker;

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        status,
        open_screen_recording_settings,
        turn_off_for_meeting,
        snapshots,
        snapshot_folder,
        delete_snapshot,
        capture_choices,
        capture_source,
        consent_reminder,
        stop_consent_reminders
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

/// The folder holding the meeting's snapshot PNG files (inside the recordings
/// folder, which the asset protocol may read), for the Images strip.
#[tauri::command]
#[specta::specta]
async fn snapshot_folder(
    screen: State<'_, Arc<ScreenService>>,
    meeting_id: MeetingId,
) -> Result<String, AppError> {
    Ok(screen
        .snapshot_folder(&meeting_id)
        .to_string_lossy()
        .into_owned())
}

/// Deletes one snapshot, its row and its PNG; false when it was gone.
#[tauri::command]
#[specta::specta]
async fn delete_snapshot(
    screen: State<'_, Arc<ScreenService>>,
    meeting_id: MeetingId,
    snapshot_id: i64,
) -> Result<bool, AppError> {
    screen
        .delete_snapshot(&meeting_id, snapshot_id)
        .await
        .into_ipc()
}

/// The record panel's picker: the displays and the other apps' windows.
#[tauri::command]
#[specta::specta]
async fn capture_choices(
    capture: State<'_, Arc<ScreenCapture>>,
) -> Result<Vec<TargetChoice>, AppError> {
    capture.choices().into_ipc()
}

/// Captures `source` for the recording of `meeting_id` from now on; false
/// when Settings keeps neither video nor snapshots.
#[tauri::command]
#[specta::specta]
async fn capture_source(
    picker: State<'_, Arc<ScreenPicker>>,
    meeting_id: MeetingId,
    source: Source,
) -> Result<bool, AppError> {
    picker.pick(&meeting_id, source).await
}

/// Whether the capture of `meeting_id` shows the consent reminder (the
/// first three captures).
#[tauri::command]
#[specta::specta]
async fn consent_reminder(
    reminders: State<'_, Arc<ConsentReminders>>,
    meeting_id: MeetingId,
) -> Result<bool, AppError> {
    Ok(reminders.remind(&meeting_id).await)
}

/// "Don't show again" on the screen capture reminder.
#[tauri::command]
#[specta::specta]
async fn stop_consent_reminders(
    reminders: State<'_, Arc<ConsentReminders>>,
) -> Result<(), AppError> {
    reminders.stop().await.into_ipc()
}
