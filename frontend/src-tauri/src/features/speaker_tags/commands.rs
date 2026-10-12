//! Commands of the `speaker-tags` plugin: call one service method, map the
//! error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_domain::AppError;
use deino_speaker_tags::{AppTagStatus, SpeakerTags};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![status, open_accessibility_settings]
}

/// Each meeting app's row for Settings → Speaker tags: Zoom, Teams, Meet.
#[tauri::command]
#[specta::specta]
async fn status(tags: State<'_, Arc<SpeakerTags>>) -> Result<Vec<AppTagStatus>, AppError> {
    Ok(tags.statuses())
}

/// Opens System Settings → Privacy & Security → Accessibility. Fails with
/// `unavailable` where there is no such page (off macOS).
#[tauri::command]
#[specta::specta]
async fn open_accessibility_settings() -> Result<(), AppError> {
    deino_speaker_tags::open_accessibility_settings().into_ipc()
}
