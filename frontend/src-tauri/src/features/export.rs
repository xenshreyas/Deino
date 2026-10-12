//! `export` feature (EXP-01, EXP-02): the IPC surface of
//! `deino_core::export`, exposed as the inline plugin `plugin:export|<command>`.
//!
//! - `commands.rs`: preview, save, copy, email, the auto-export folder
//!   picker and the account export (start, cancel, status), thin over
//!   `ExportService`, `AutoExportService` and `FullExportService`;
//! - `events.rs`: the typed `FullExportProgress` event and its adapter;
//! - `platform.rs`: the OS adapters behind the core's ports: the save and
//!   folder dialogs (`tauri-plugin-dialog`), the clipboard (`arboard`, HTML
//!   with a plain-text fallback) and the URL opener (`open`, for `mailto:`).
//!
//! Every path comes from a dialog in Rust: no command takes a path from the
//! webview. The services are built in `composition/export.rs`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;
mod platform;

pub use events::{FullExportProgress, TauriFullExportEvents};
pub use platform::{SystemClipboard, SystemOpener, TauriDialogs};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "export";

super::registry::register_plugin!(Guarded);

/// The plugin's commands and events, for the invoke handler, event
/// registration and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![FullExportProgress])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `export` plugin.
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
