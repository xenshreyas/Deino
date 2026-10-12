//! `speaker-tags` feature (CAP-05): names from the meeting app's window,
//! exposed as the inline plugin `plugin:speaker-tags|<command>`.
//!
//! - `status`: one row per app (Zoom, Teams, Meet) for Settings → Speaker
//!   tags: Enabled, Needs permission or Unsupported;
//! - `open_accessibility_settings`: opens System Settings → Privacy &
//!   Security → Accessibility, where the user grants it. Deino never
//!   changes the setting and never shows the system prompt.
//! - [`WatchOnStart`]: the recorder's start observer that watches each
//!   recording's meeting app.
//!
//! `deino-speaker-tags` does the work; `composition/speaker_tags.rs` wires
//! it. The status is live state, not a table, so the UI polls it while the
//! page is open; the plugin declares no events.

mod commands;
#[cfg(test)]
mod contract_tests;
mod start_watch;

pub use start_watch::WatchOnStart;
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "speaker-tags";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `speaker-tags` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
