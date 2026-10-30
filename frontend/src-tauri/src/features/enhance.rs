//! `enhance` feature (NOT-02, NOT-03): the IPC surface of
//! `deino_core::enhance`, exposed as the inline plugin
//! `plugin:enhance|<command>`.
//!
//! The Notes tab's Enhanced view reads a version (`get`) and the version
//! history (`versions`), writes notes (`enhance`, which asks before
//! replacing the user's edits), saves edits (`save`), restores an old
//! version (`restore`) and opens citations (`cited`). While notes are
//! written it shows them as they arrive: the [`NotesRun`] event, the
//! latest draft (`draft`) and Stop (`stop`). The service is wired in
//! `composition/enhance.rs`. Saved versions reach the UI through
//! `DbChanged` on `enhanced_notes`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod events;

pub use events::{NotesRun, NotesRunUpdate, TauriNotesProgress};

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "enhance";

super::registry::register_plugin!(Guarded);

/// The plugin's commands, for the invoke handler and the TypeScript bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![NotesRun])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `enhance` plugin.
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
