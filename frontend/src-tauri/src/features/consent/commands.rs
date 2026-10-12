//! Commands of the `consent` plugin: call one service method, map the
//! error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::settings::SettingsService;
use deino_domain::AppError;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![reminder, stop_reminding]
}

/// Whether the consent reminder shows when a recording starts.
#[tauri::command]
#[specta::specta]
async fn reminder(settings: State<'_, Arc<SettingsService>>) -> Result<bool, AppError> {
    Ok(settings.get().await.consent.reminder)
}

/// "Don't show again": turns the consent reminder off.
#[tauri::command]
#[specta::specta]
async fn stop_reminding(settings: State<'_, Arc<SettingsService>>) -> Result<(), AppError> {
    settings
        .update(|settings| settings.consent.reminder = false)
        .await
        .map(drop)
        .into_ipc()
}
