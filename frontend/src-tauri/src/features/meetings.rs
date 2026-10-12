//! `meetings` feature: the IPC surface of `deino_core::meetings`, exposed as
//! the inline plugin `plugin:meetings|<command>`: opening and renaming a
//! meeting, and the Meetings page's list, views, trash, bulk actions and
//! merge (ORG-07).
//!
//! The commands are thin: they take the services from Tauri state
//! (wired in `composition.rs`), call one method and map errors with
//! `into_ipc`. Changes reach the UI through `DbChanged` (`features/db.rs`),
//! so this plugin declares no events of its own.

mod commands;
#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod list_contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "meetings";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `meetings` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
