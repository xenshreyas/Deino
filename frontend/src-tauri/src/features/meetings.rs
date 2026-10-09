//! `meetings` feature: the IPC surface of `deino_core::meetings`, exposed as
//! the inline plugin `plugin:meetings|<command>`.
//!
//! The commands are thin: they take the `MeetingService` from Tauri state
//! (wired in `composition.rs`), call one method and map errors with
//! `into_ipc`. Changes reach the UI through `DbChanged` (`features/db.rs`),
//! so this plugin declares no events of its own.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "meetings";

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
