//! `hotkeys` feature (DEV-05): the system-wide hotkeys of Settings →
//! Shortcuts, exposed as the inline plugin `plugin:hotkeys|<command>`.
//!
//! - `commands.rs`: `list` and `set` over `deino_core::hotkeys::HotkeyService`;
//! - [`registrar`]: the service's `HotkeyRegistrar` over
//!   `tauri-plugin-global-shortcut`, and the table that turns a pressed
//!   hotkey back into its action.
//!
//! `composition/hotkeys.rs` registers the global-shortcut plugin, whose
//! handler runs each action's intent through the link service, and
//! registers the saved hotkeys at startup.

mod commands;
#[cfg(test)]
mod contract_tests;
pub(crate) mod registrar;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "hotkeys";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `hotkeys` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .build()
}
