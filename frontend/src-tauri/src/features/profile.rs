//! `profile` feature (NOT-05): saving Settings → Profile through
//! `deino_core::profile::ProfileService`, which also keeps the "Me" person
//! in step, exposed as the inline plugin `plugin:profile|<command>`.
//!
//! The UI reads the profile with the settings (`useSettings`); a save arrives
//! there as `SettingsChanged`, and the person write as `DbChanged` (`people`).

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "profile";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `profile` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
