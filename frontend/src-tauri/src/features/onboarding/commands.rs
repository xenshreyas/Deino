//! Commands of the `onboarding` plugin: parse input, call one service
//! method, map the error. No logic lives here. They are async so the
//! permission prompts never block the main thread.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::permissions::{PermissionKind, PermissionService, PermissionStatus};
use deino_domain::AppError;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![permissions, request_permission, open_permission_settings]
}

/// Every permission onboarding asks for, read without prompting.
#[tauri::command]
#[specta::specta]
async fn permissions(
    permissions: State<'_, Arc<PermissionService>>,
) -> Result<Vec<PermissionStatus>, AppError> {
    Ok(permissions.statuses())
}

/// Asks macOS for `kind` (showing its prompt when it has not been answered)
/// and returns the permission's status afterwards.
#[tauri::command]
#[specta::specta]
async fn request_permission(
    permissions: State<'_, Arc<PermissionService>>,
    kind: PermissionKind,
) -> Result<PermissionStatus, AppError> {
    Ok(permissions.request(kind).await)
}

/// Opens the System Settings page where `kind` is granted.
#[tauri::command]
#[specta::specta]
async fn open_permission_settings(
    permissions: State<'_, Arc<PermissionService>>,
    kind: PermissionKind,
) -> Result<(), AppError> {
    permissions.open_settings(kind).into_ipc()
}
