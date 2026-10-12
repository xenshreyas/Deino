//! Commands of the `clips` plugin: parse, call the service, map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::clips::{Clip, ClipService, NewClip};
use deino_domain::{AppError, ClipId, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![create, list, list_all, rename, delete, reveal]
}

/// Cuts a named range of the meeting's recording into a clip.
#[tauri::command]
#[specta::specta]
async fn create(clips: State<'_, Arc<ClipService>>, clip: NewClip) -> Result<Clip, AppError> {
    clips.create(clip).await.into_ipc()
}

/// The meeting's clips, in time order.
#[tauri::command]
#[specta::specta]
async fn list(
    clips: State<'_, Arc<ClipService>>,
    meeting_id: MeetingId,
) -> Result<Vec<Clip>, AppError> {
    clips.list(&meeting_id).await.into_ipc()
}

/// Every clip of every meeting not in the trash, newest first.
#[tauri::command]
#[specta::specta]
async fn list_all(clips: State<'_, Arc<ClipService>>) -> Result<Vec<Clip>, AppError> {
    clips.list_all().await.into_ipc()
}

/// Renames the clip and its file.
#[tauri::command]
#[specta::specta]
async fn rename(
    clips: State<'_, Arc<ClipService>>,
    id: ClipId,
    name: String,
) -> Result<Clip, AppError> {
    clips.rename(&id, &name).await.into_ipc()
}

/// Deletes the clip and its file.
#[tauri::command]
#[specta::specta]
async fn delete(clips: State<'_, Arc<ClipService>>, id: ClipId) -> Result<(), AppError> {
    clips.delete(&id).await.into_ipc()
}

/// Shows the clip's file in Finder.
#[tauri::command]
#[specta::specta]
async fn reveal(clips: State<'_, Arc<ClipService>>, id: ClipId) -> Result<(), AppError> {
    clips.reveal(&id).await.into_ipc()
}
