//! `retention` feature: the IPC surface of `deino_core::retention` (PRV-01),
//! exposed as the inline plugin `plugin:retention|<command>`.
//!
//! - `commands.rs`: what retention removed from a meeting, for the meeting
//!   page's "Transcript deleted" and the disabled Regenerate; thin over
//!   `RetentionService` (wired in `composition/retention.rs`).
//!
//! The periods are settings (`privacy.retention`), changed through the
//! settings plugin. The sweep runs with the trash purge; its writes go
//! through the database writer, so `DbChanged` on `meetings` keeps the UI
//! fresh. There are no feature events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "retention";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript
/// bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `retention` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .build()
}
