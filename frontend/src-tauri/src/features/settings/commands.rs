//! Commands of the `settings` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::settings::{Settings, SettingsPatch, SettingsService};
use deino_domain::AppError;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![get, patch]
}

/// Every setting.
#[tauri::command]
#[specta::specta]
async fn get(settings: State<'_, Arc<SettingsService>>) -> Result<Settings, AppError> {
    Ok(settings.get().await)
}

/// Applies an RFC 7396 merge patch (only the fields that change; `null`
/// resets one to its default) and returns every setting. Fails with
/// `invalid`, changing nothing, for an unknown setting, a wrong type or a
/// value that breaks a rule.
#[tauri::command]
#[specta::specta]
async fn patch(
    settings: State<'_, Arc<SettingsService>>,
    patch: serde_json::Value,
) -> Result<Settings, AppError> {
    let patch = SettingsPatch::parse(patch).into_ipc()?;
    settings.patch(&patch).await.into_ipc()
}
