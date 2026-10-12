//! `people` feature: the IPC surface of people, exposed as the inline plugin
//! `plugin:people|<command>`.
//!
//! - `search` feeds the notepad's @mention suggestions (NOT-01) and
//!   `from_mention` adds someone new from one;
//! - the People list and page, adding, editing, notes, merging and the AI
//!   summary refresh (ORG-04, `deino_core::entities::PersonService`);
//! - a meeting's attendees for the meeting header, added and removed by
//!   hand (`AttendanceService`).
//!
//! Every service is built in `composition/entities.rs`.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "people";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `people` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
