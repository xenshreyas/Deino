//! `companies` feature: the IPC surface of companies (ORG-04), exposed as
//! the inline plugin `plugin:companies|<command>`: the Companies list and
//! page, adding, editing, notes, merging, the AI summary refresh and the
//! logo fetch, thin over `deino_core::entities::{CompanyService,
//! EntitySummarizer, LogoService}` (built in `composition/entities.rs`).

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "companies";

/// The plugin's commands, for the invoke handler and the bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `companies` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
