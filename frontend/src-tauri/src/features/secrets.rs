//! `secrets` feature: API keys in the Keychain, exposed as the inline plugin
//! `plugin:secrets|<command>`.
//!
//! The UI can save a key, ask whether one is saved, and forget it. It can
//! never read a key back: there is no `get` command, and `deino_secrets`
//! values cannot be serialized. Rust consumers (the LLM clients) read keys
//! through [`stored_key`] or the `Arc<dyn SecretStore>` in Tauri state.

mod commands;
#[cfg(test)]
mod contract_tests;

use std::sync::Arc;

use deino_secrets::{SecretError, SecretName, SecretStore, SecretValue};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Manager, Runtime};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "secrets";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `secrets` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}

/// The saved key `name`, for legacy Rust code that has only an `AppHandle`.
/// `Ok(None)` when no key is saved.
pub(crate) async fn stored_key<R: Runtime>(
    app: &AppHandle<R>,
    name: SecretName,
) -> Result<Option<SecretValue>, SecretError> {
    let Some(secrets) = app.try_state::<Arc<dyn SecretStore>>() else {
        log::warn!(target: "secrets", "a key was read before startup wired the Keychain");
        return Ok(None);
    };
    secrets.get(name).await
}

/// `typed` when the user typed a key, else the saved key `name`: the legacy
/// commands that test a key accept one from the form or fall back to the
/// Keychain, since the UI cannot read saved keys.
pub(crate) async fn typed_or_stored<R: Runtime>(
    app: &AppHandle<R>,
    typed: Option<String>,
    name: SecretName,
) -> Result<Option<String>, SecretError> {
    if let Some(key) = typed.filter(|key| !key.trim().is_empty()) {
        return Ok(Some(key));
    }
    let stored = stored_key(app, name).await?;
    Ok(stored.map(|value| value.expose().to_owned()))
}
