//! `player` feature: the IPC surface of `deino_core::player` (TRN-03),
//! exposed as the inline plugin `plugin:player|<command>`.
//!
//! - `media`: the meeting's playable recording files (mixed, Me, Them, a
//!   screen recording), each a path inside the recordings folder that the
//!   UI loads through the asset protocol. `tauri.conf.json` scopes that
//!   protocol to `$APPDATA/recordings/**` (and the notepad's images under
//!   `$APPDATA/meetings/**`), so nothing else on disk can be read.
//! - `download`: copies one file where the user chooses in the save dialog,
//!   which Rust opens; no command takes a path from the UI.
//!
//! The service is wired in `composition.rs`. Changes to a meeting's files
//! (the `finalize_audio` stage) reach the UI through `DbChanged`
//! (`meetings`), so the plugin declares no events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "player";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `player` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
