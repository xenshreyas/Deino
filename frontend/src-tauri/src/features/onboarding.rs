//! `onboarding` feature (FND-11): the permissions first-run setup asks for,
//! through `deino_core::permissions::PermissionService`, and the bundled
//! sample meeting (`deino_core::sample::SampleImporter`), exposed as the
//! inline plugin `plugin:onboarding|<command>`.
//!
//! The flow itself is UI (`frontend/src/features/onboarding/`): its step
//! machine saves progress in the typed settings (`Settings::onboarding`)
//! through the `settings` plugin, and every other step calls the plugin
//! that owns it (transcription, search, secrets, llm, calendar, profile,
//! recording).

mod commands;
#[cfg(test)]
mod contract_tests;

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "onboarding";

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `onboarding` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}
