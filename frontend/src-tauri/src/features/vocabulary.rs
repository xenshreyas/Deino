//! `vocabulary` feature: the IPC surface of `deino_core::vocabulary`
//! (TRN-05), exposed as the inline plugin `plugin:vocabulary|<command>`.
//!
//! - Settings → Vocabulary: `list`, `create`, `update`, `delete` (terms with
//!   their sounds-like spellings, and replacement rules);
//! - a meeting's corrections: `corrections` and `undo` (all of them, or one
//!   segment's).
//!
//! The service is wired in `composition/vocabulary.rs`. Changes reach the UI
//! through `DbChanged` (`vocab_terms`, `vocab_variants`, `vocab_corrections`,
//! `transcript_segments`), so the plugin declares no events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "vocabulary";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `vocabulary` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
