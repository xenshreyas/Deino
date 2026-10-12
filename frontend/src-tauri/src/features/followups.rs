//! `followups` feature: the IPC surface of `deino_core::followups` (NOT-07),
//! exposed as the inline plugin `plugin:followups|<command>`.
//!
//! - `commands.rs`: Home's suggestions, a meeting's draft, saving an edit,
//!   dismissing (for good), opening in Mail and copying; thin over
//!   `FollowUpService` (wired in `composition/followups.rs`).
//!
//! Drafts are written by the `follow_up_draft` pipeline stage through the
//! database writer, so `DbChanged` on `follow_up_drafts` keeps the UI
//! fresh; there are no feature events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "followups";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript
/// bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `followups` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .build()
}
