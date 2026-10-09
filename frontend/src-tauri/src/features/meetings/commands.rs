//! Commands of the `meetings` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::meetings::{Meeting, MeetingService};
use deino_domain::{AppError, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![list, get, rename]
}

/// Every meeting, newest first.
#[tauri::command]
#[specta::specta]
async fn list(meetings: State<'_, Arc<MeetingService>>) -> Result<Vec<Meeting>, AppError> {
    meetings.list().await.into_ipc()
}

/// The meeting with `id`; fails with `not_found` if there is none.
#[tauri::command]
#[specta::specta]
async fn get(meetings: State<'_, Arc<MeetingService>>, id: MeetingId) -> Result<Meeting, AppError> {
    meetings.get(&id).await.into_ipc()
}

/// Renames the meeting and returns it; fails with `invalid` for a blank or
/// overlong title and `not_found` for an unknown `id`.
#[tauri::command]
#[specta::specta]
async fn rename(
    meetings: State<'_, Arc<MeetingService>>,
    id: MeetingId,
    title: String,
) -> Result<Meeting, AppError> {
    meetings.rename(&id, &title).await.into_ipc()
}
