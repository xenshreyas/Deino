//! `settings` feature: the IPC surface of `deino_core::settings`, exposed as
//! the inline plugin `plugin:settings|<command>`.
//!
//! - `get` returns the whole [`Settings`] document;
//! - `patch` takes an RFC 7396 merge patch, the only untyped JSON that
//!   reaches the core, which parses it back into [`Settings`] or rejects it
//!   with `invalid`;
//! - [`SettingsChanged`] carries every saved change, so the UI's
//!   `useSettings()` cache never goes stale (settings are a file, not a
//!   table, so `DbChanged` does not cover them).
//!
//! Legacy Rust code reads the current settings with [`current`].

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

use std::sync::Arc;

use deino_core::settings::{Settings, SettingsService};
pub use events::{SettingsChanged, TauriSettingsEvents};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Manager, Runtime};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "settings";

super::registry::register_plugin!(Open);

/// The plugin's commands and events, for the invoke handler and the bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![SettingsChanged])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `settings` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app, _| {
            builder.mount_events(app);
            Ok(())
        })
        .build()
}

/// The current settings, for legacy code that has only an `AppHandle`.
///
/// The composition root manages the service before anything else starts,
/// so the defaults are only a guard for a broken startup.
pub(crate) async fn current<R: Runtime>(app: &AppHandle<R>) -> Settings {
    let Some(settings) = app.try_state::<Arc<SettingsService>>() else {
        log::warn!(target: "settings", "settings read before startup wired them; using defaults");
        return Settings::default();
    };
    settings.get().await
}
