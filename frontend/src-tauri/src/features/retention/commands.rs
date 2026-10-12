//! Commands of the `retention` plugin: parse input, call one service
//! method, map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::retention::{MeetingRetention, RetentionService};
use deino_core::settings::SettingsService;
use deino_domain::{AppError, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![status, pause_tip, pause_tip_seen]
}

/// What retention removed from a meeting: when its recording and its
/// transcript were deleted, each `null` while kept.
#[tauri::command]
#[specta::specta]
async fn status(
    retention: State<'_, Arc<RetentionService>>,
    meeting: MeetingId,
) -> Result<MeetingRetention, AppError> {
    retention.status(&meeting).await.into_ipc()
}

/// Whether the record panel shows the "pause for sensitive discussions" tip:
/// until it has been seen once. The panel reads only this, never the whole
/// settings document (least privilege, as CAP-13's consent reminder).
#[tauri::command]
#[specta::specta]
async fn pause_tip(settings: State<'_, Arc<SettingsService>>) -> Result<bool, AppError> {
    Ok(!settings.get().await.privacy.pause_tip_seen)
}

/// Records that the pause tip was seen, so it never shows again.
#[tauri::command]
#[specta::specta]
async fn pause_tip_seen(settings: State<'_, Arc<SettingsService>>) -> Result<(), AppError> {
    settings
        .update(|settings| settings.privacy.pause_tip_seen = true)
        .await
        .map(drop)
        .into_ipc()
}
