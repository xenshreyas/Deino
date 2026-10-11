//! `transcript` feature: the IPC surface of `deino_core::transcript`, exposed
//! as the inline plugin `plugin:transcript|<command>` (TRN-01).
//!
//! `get` serves the transcript view on the meeting page and the record
//! screen: the saved segments with their speakers' names. Saved changes
//! reach the UI through `DbChanged` (`transcript_segments`, `speakers`,
//! `people`); lines still being spoken arrive as the recorder's
//! `LiveSegment` events. TRN-02 adds the segment edits here.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "transcript";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `transcript` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
