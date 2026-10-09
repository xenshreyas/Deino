//! Commands of the `profile` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::profile::{ProfileInput, ProfileService};
use deino_core::settings::ProfileSettings;
use deino_domain::AppError;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![update, context]
}

/// Normalises and saves the profile, updates the "Me" person and returns
/// the saved profile; `invalid` names a malformed address by position or a
/// field over its limit.
#[tauri::command]
#[specta::specta]
async fn update(
    profile: State<'_, Arc<ProfileService>>,
    input: ProfileInput,
) -> Result<ProfileSettings, AppError> {
    profile.update(input).await.into_ipc()
}

/// The context every LLM prompt starts with, or `null` for an empty profile.
#[tauri::command]
#[specta::specta]
async fn context(profile: State<'_, Arc<ProfileService>>) -> Result<Option<String>, AppError> {
    Ok(profile.context().await)
}
