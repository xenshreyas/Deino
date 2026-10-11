//! `transcription` feature: the Parakeet Unified model manager, exposed as
//! the inline plugin `plugin:transcription|<command>`.
//!
//! Commands list, download, cancel and delete model files through
//! `deino_core::models::ModelService`. Events carry download progress and
//! model state changes. The engine itself is `Settings::transcription`; the
//! live transcript's events belong to the `recording` plugin.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{ModelChanged, ModelDownloadProgress, TauriModelEvents};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "transcription";

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![
            ModelChanged,
            ModelDownloadProgress
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `transcription` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let builder = specta();
    Builder::new(NAME)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app, _| {
            builder.mount_events(app);
            Ok(())
        })
        .build()
}
