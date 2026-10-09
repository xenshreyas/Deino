//! `notes` feature: the notepad's IPC (NOT-01), exposed as the inline plugin
//! `plugin:notes|<command>`.
//!
//! - `get`, `save`: a meeting's notes as a TipTap document; the UI autosaves
//!   about once a second;
//! - `create_scheduled`: a meeting for notes written before a recording;
//! - `adopt`: moves the record screen's draft into the meeting a recording saved;
//! - `save_image`: a pasted image's bytes (base64), checked and stored by Rust;
//! - `attach_dropped_image`: a file dropped on the window, by path, accepted
//!   only if the window really received that drop ([`DroppedFiles`]);
//! - `markdown_to_doc`, `doc_to_markdown`: `deino-doc` for the summary view.
//!
//! [`NotesSaved`] tells other windows a meeting's notes changed; queries
//! stay fresh through `DbChanged` on `meeting_notes`.

mod commands;
#[cfg(test)]
mod contract_tests;
mod dropped;
mod events;

use std::sync::Arc;

use deino_core::notes::NoteService;
use deino_domain::MeetingId;
pub use dropped::DroppedFiles;
pub use events::{NotesSaved, TauriNotesEvents};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Manager, Runtime};

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "notes";

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

/// The meeting's notes as Readable Markdown, the legacy summary's user
/// context until NOT-02 (the notepad replaced the "Add context for AI
/// summary" box). `None` when there are no notes or they cannot be read; a
/// summary never fails because of notes.
pub(crate) async fn summary_context<R: Runtime>(
    app: &AppHandle<R>,
    meeting: &str,
) -> Option<String> {
    let notes = app.try_state::<Arc<NoteService>>()?;
    match notes.markdown(&MeetingId::new(meeting)).await {
        Ok(markdown) => markdown,
        Err(error) => {
            log::warn!(target: "notes", "summary for {meeting} runs without its notes: {error}");
            None
        }
    }
}
