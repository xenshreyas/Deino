//! `clips` feature: the IPC surface of `deino_core::clips` (TRN-04, ADR
//! 0048), exposed as the inline plugin `plugin:clips|<command>`.
//!
//! - `create`: cuts a named range of the meeting's recording into
//!   `<recording folder>/clips/<name>.m4a` (`.mp4` with a screen recording).
//! - `list` (a meeting's, in time order) and `list_all` (every clip, newest
//!   first, for ⌘K); a trashed meeting's clips are never listed.
//! - `rename`, `delete`, and `reveal` (Finder, only for a file inside the
//!   recordings folder). No command takes a path from the UI.
//!
//! The cutter is `deino_audio::clip` ([`platform::AvClipCutter`]). Changes
//! reach the UI through `DbChanged` (`clips`), so the plugin declares no
//! events.

mod commands;
#[cfg(test)]
mod contract_tests;
pub(crate) mod platform;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "clips";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `clips` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
