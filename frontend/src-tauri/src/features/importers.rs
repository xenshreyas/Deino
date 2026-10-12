//! `importers` feature (CAP-12): Settings → Import, the IPC surface of
//! `deino_core::importers` and `deino-importers`, exposed as the inline
//! plugin `plugin:importers|<command>`.
//!
//! - Keys: `sources` (each source and whether its key is saved), `save_key`
//!   and `forget_key` (the Keychain; saving switches the provider's network
//!   service on). The UI never reads a key back.
//! - A folder: `pick_folder` opens the folder dialog in Rust.
//! - The job: `preview` (the count first), `start` (runs in the background),
//!   `cancel` and `status`; progress arrives as `import-progress-changed`
//!   events, and the new meetings through `DbChanged`.
//!
//! The services are wired in `composition/importers.rs`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{ImportProgressChanged, TauriImportEvents};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "importers";

super::registry::register_plugin!(Guarded);

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![ImportProgressChanged])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `importers` plugin.
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
