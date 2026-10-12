//! Commands of the `api` plugin: parse input, call one service method, map
//! the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_api::{ApiController, ApiStatus};
use deino_core::api_keys::{ApiKey, ApiKeyService, CreatedApiKey, NewApiKey};
use deino_core::settings::ApiSettings;
use deino_domain::{ApiKeyId, AppError};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        server_status,
        configure_server,
        list_keys,
        create_key,
        revoke_key,
        delete_key
    ]
}

/// The setting, and the base URL while the server listens.
#[tauri::command]
#[specta::specta]
async fn server_status(api: State<'_, Arc<ApiController>>) -> Result<ApiStatus, AppError> {
    Ok(api.status().await)
}

/// Saves the switch and port and starts, stops or moves the server.
#[tauri::command]
#[specta::specta]
async fn configure_server(
    api: State<'_, Arc<ApiController>>,
    settings: ApiSettings,
) -> Result<ApiStatus, AppError> {
    api.configure(settings).await.into_ipc()
}

/// Every key, newest first, revoked ones included.
#[tauri::command]
#[specta::specta]
async fn list_keys(keys: State<'_, Arc<ApiKeyService>>) -> Result<Vec<ApiKey>, AppError> {
    keys.list().await.into_ipc()
}

/// Makes a key; the token in the reply is never shown again.
#[tauri::command]
#[specta::specta]
async fn create_key(
    keys: State<'_, Arc<ApiKeyService>>,
    key: NewApiKey,
) -> Result<CreatedApiKey, AppError> {
    keys.create(&key).await.into_ipc()
}

/// Revokes a key at once.
#[tauri::command]
#[specta::specta]
async fn revoke_key(keys: State<'_, Arc<ApiKeyService>>, id: ApiKeyId) -> Result<ApiKey, AppError> {
    keys.revoke(&id).await.into_ipc()
}

/// Removes a key from the list.
#[tauri::command]
#[specta::specta]
async fn delete_key(keys: State<'_, Arc<ApiKeyService>>, id: ApiKeyId) -> Result<(), AppError> {
    keys.delete(&id).await.into_ipc()
}
