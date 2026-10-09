//! `folders` feature: the IPC surface of `deino_core::folders` (ORG-05),
//! exposed as the inline plugin `plugin:folders|<command>`.
//!
//! The commands are thin: they take `FolderService` (the tree) or
//! `FilingService` (meetings in folders and auto-add rules) from Tauri
//! state, wired in `composition/organize.rs`, call one method and map
//! errors with `into_ipc`. Changes reach the UI through `DbChanged`
//! (`folders`, `meeting_folders`, `folder_rules`), so the plugin declares
//! no events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "folders";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `folders` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
