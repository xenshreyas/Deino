//! `screen` feature (CAP-09): the meeting window's video and snapshots,
//! exposed as the inline plugin `plugin:screen|<command>`.
//!
//! - `status`: whether this Mac can capture, whether Screen Recording is
//!   granted (read without prompting), and what the capture of the current
//!   recording is doing, for Settings → Screen capture and the record panel;
//! - `open_screen_recording_settings`: opens System Settings → Privacy &
//!   Security → Screen Recording. Deino never changes the setting;
//! - `turn_off_for_meeting`: "Turn off for this meeting": stops the
//!   meeting's capture and keeps it off;
//! - `snapshots`: a meeting's kept snapshots, in time order;
//! - [`CaptureOnStart`]: the recorder's start observer that starts a
//!   capture when the settings, the detected meeting app and the
//!   permission allow it.
//!
//! `deino-screen` captures, `deino-core::screen` stores; the wiring is
//! `composition/screen.rs`. The status is live state, so the UI polls it;
//! snapshots are a table (`snapshots`), so `DbChanged` keeps them fresh.

mod commands;
#[cfg(test)]
mod contract_tests;
mod start_capture;

pub use start_capture::CaptureOnStart;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "screen";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `screen` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
