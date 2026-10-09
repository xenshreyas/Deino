//! Commands of the `transcription` plugin: parse input, call one service
//! method, map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::models::{ModelFileId, ModelFileView, ModelService};
use deino_domain::AppError;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![list_models, download_model, cancel_download, delete_model]
}

/// Every Parakeet Unified file with its state here; the recommended one is flagged.
#[tauri::command]
#[specta::specta]
async fn list_models(models: State<'_, Arc<ModelService>>) -> Result<Vec<ModelFileView>, AppError> {
    models.list().await.into_ipc()
}

/// Downloads a file, resuming any partial, and verifies its SHA-256.
/// Resolves when it is ready; fails with `cancelled`, `network` or `conflict`.
#[tauri::command]
#[specta::specta]
async fn download_model(
    models: State<'_, Arc<ModelService>>,
    id: ModelFileId,
) -> Result<(), AppError> {
    models.download(&id).await.into_ipc()
}

/// Stops a running download, keeping what arrived; false if none ran.
#[tauri::command]
#[specta::specta]
async fn cancel_download(
    models: State<'_, Arc<ModelService>>,
    id: ModelFileId,
) -> Result<bool, AppError> {
    Ok(models.cancel(&id))
}

/// Deletes a downloaded or partial file.
#[tauri::command]
#[specta::specta]
async fn delete_model(
    models: State<'_, Arc<ModelService>>,
    id: ModelFileId,
) -> Result<(), AppError> {
    models.delete(&id).await.into_ipc()
}
