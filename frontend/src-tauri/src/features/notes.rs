//! `notes` feature: the notepad's IPC (NOT-01), exposed as the inline plugin
//! `plugin:notes|<command>`.
//!
//! - `get`, `save`: a meeting's notes as a TipTap document; the UI autosaves
//!   about once a second;
//! - `create_scheduled`: a meeting for notes written before a recording;
//! - `save_image`: a pasted image's bytes (base64), checked and stored by Rust;
//! - `attach_dropped_image`: a file dropped on the window, by path, accepted
//!   only if the window really received that drop ([`DroppedFiles`]);
//! - `markdown_to_doc`, `doc_to_markdown`: `deino-doc` for views that hold Markdown.
//!
//! [`NotesSaved`] tells other windows a meeting's notes changed; queries
//! stay fresh through `DbChanged` on `meeting_notes`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod dropped;
mod events;

pub use dropped::DroppedFiles;
pub use events::{NotesSaved, TauriNotesEvents};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin name: its commands are `plugin:<NAME>|<command>`.
pub const NAME: &str = "notes";

super::registry::register_plugin!(Guarded);

/// The plugin's commands and events, for the invoke handler and the bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(commands::collect())
        .events(tauri_specta::collect_events![NotesSaved])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `notes` plugin.
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
