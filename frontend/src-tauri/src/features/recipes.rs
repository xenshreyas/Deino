//! `recipes` feature: the IPC surface of `deino_core::recipes` (AI-04),
//! exposed as the inline plugin `plugin:recipes|<command>`.
//!
//! `commands.rs` lists, creates, edits, remixes and deletes recipes, and
//! exports, imports and shares them; thin over `RecipeService` (wired in
//! `composition/ask.rs`, which also gives it to the ask service). Running a
//! recipe is the `ask` plugin's. Writes go through the database writer, so
//! `DbChanged` on `recipes` keeps the UI fresh; there are no feature events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "recipes";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript
/// bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `recipes` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .build()
}
