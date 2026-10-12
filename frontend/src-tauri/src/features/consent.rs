//! `consent` feature (CAP-13): the consent reminder's two questions for the
//! record panel, exposed as the inline plugin `plugin:consent|<command>`.
//!
//! - `reminder`: whether the reminder card shows when a recording starts
//!   (`consent.reminder`);
//! - `stop_reminding`: "Don't show again", which turns it off.
//!
//! The record panel's window may call only these two (its capability grants
//! `consent:default`), never the whole settings document. The chat notice
//! has no commands: it is wired in `composition/consent.rs` and set in
//! Settings → Consent through the settings plugin.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "consent";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `consent` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
