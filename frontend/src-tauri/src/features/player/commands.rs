//! Commands of the `player` plugin: parse, call the service, map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::player::{DownloadOutcome, MediaSource, MeetingMedia, PlayerService};
use deino_domain::{AppError, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![media, download]
}

/// The meeting's playable recording files; none for a meeting without a
/// recording on this Mac.
#[tauri::command]
#[specta::specta]
async fn media(
    player: State<'_, Arc<PlayerService>>,
    meeting_id: MeetingId,
) -> Result<MeetingMedia, AppError> {
    player.media(&meeting_id).await.into_ipc()
}

/// Saves a copy of one recording file where the user chooses.
#[tauri::command]
#[specta::specta]
async fn download(
    player: State<'_, Arc<PlayerService>>,
    meeting_id: MeetingId,
    source: MediaSource,
) -> Result<DownloadOutcome, AppError> {
    player.download(&meeting_id, source).await.into_ipc()
}
