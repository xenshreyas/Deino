//! Commands of the `secrets` plugin: parse input, call the store, map the
//! error. The store is the `SecretStore` port itself; there is no rule above
//! it worth a service.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_domain::AppError;
use deino_secrets::{SecretName, SecretStatus, SecretStore, SecretValue};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![set, status, delete]
}

/// Saves `value` as the key `name`, replacing any saved key. Fails with
/// `invalid` for a blank key and `unavailable` when the Keychain is.
#[tauri::command]
#[specta::specta]
async fn set(
    secrets: State<'_, Arc<dyn SecretStore>>,
    name: SecretName,
    value: String,
) -> Result<(), AppError> {
    let value = SecretValue::parse(&value).into_ipc()?;
    secrets.set(name, value).await.into_ipc()
}

/// Whether the key `name` is saved: `missing`, `stored`, or `locked` when
/// macOS refused access and the user should enter it again. Never the key.
#[tauri::command]
#[specta::specta]
async fn status(
    secrets: State<'_, Arc<dyn SecretStore>>,
    name: SecretName,
) -> Result<SecretStatus, AppError> {
    secrets.status(name).await.into_ipc()
}

/// Forgets the key `name`; succeeds when none is saved.
#[tauri::command]
#[specta::specta]
async fn delete(
    secrets: State<'_, Arc<dyn SecretStore>>,
    name: SecretName,
) -> Result<(), AppError> {
    secrets.delete(name).await.into_ipc()
}
