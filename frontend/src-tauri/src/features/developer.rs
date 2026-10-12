//! `developer` feature (DEV-02/04): Settings → Developer's command-line
//! setup through `deino_core::cli_setup::CliSetupService`, exposed as the
//! inline plugin `plugin:developer|<command>`.
//!
//! The `deino` CLI ships inside the app as the `cli` sidecar
//! (`Contents/MacOS/cli`); these commands report where it is and link it
//! onto the `PATH`. Its MCP server reads the database on its own, read-only
//! (ADR 0035), so nothing here talks to it.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "developer";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `developer` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
