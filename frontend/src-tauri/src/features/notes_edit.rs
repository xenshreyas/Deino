//! `notes-edit` feature: the IPC surface of `deino_core::notes::edit`
//! (NOT-06, editing enhanced notes by asking), exposed as the inline plugin
//! `plugin:notes-edit|<command>`.
//!
//! - `commands.rs`: propose an edit, accept one; thin over
//!   `NoteEditService` (wired in `composition/notes_edit.rs`). Undo is the
//!   `enhance` plugin's `restore` of the previous version.
//!
//! An accepted edit is written through the database writer, so `DbChanged`
//! on `enhanced_notes` refreshes the notes; there are no feature events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "notes-edit";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript
/// bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `notes-edit` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .build()
}
