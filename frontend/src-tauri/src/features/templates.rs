//! `templates` feature (NOT-04): the IPC surface of `deino_core::templates`,
//! exposed as the inline plugin `plugin:templates|<command>`.
//!
//! The library commands use `TemplateService`; the per-meeting ones
//! (`meeting_template`, `set_meeting_template`, `preview`) use
//! `MeetingTemplateService`. Both are wired in `composition.rs`. Changes reach
//! the UI through `DbChanged` (`templates`, `template_sections`, `meetings`),
//! so the plugin declares no events.

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "templates";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `templates` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
