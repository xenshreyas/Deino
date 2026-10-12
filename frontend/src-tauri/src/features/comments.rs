//! `comments` feature: the IPC surface of `deino_core::comments` (TRN-06,
//! ADR 0048), exposed as the inline plugin `plugin:comments|<command>`.
//!
//! - `list`: a meeting's threads with their comments (none for a trashed
//!   meeting).
//! - `start`: a thread on a span of a transcript line (segment ID and
//!   offsets) or of the notes, with its first comment.
//! - `reply`, `edit`, `set_resolved` and `delete` (a thread goes with its
//!   last comment).
//!
//! Changes reach the UI through `DbChanged` (`comment_threads`,
//! `comments`), so the plugin declares no events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "comments";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `comments` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
