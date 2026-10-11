//! Commands of the `notes` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use deino_core::meetings::Meeting;
use deino_core::notes::{MeetingNotes, NoteImage, NoteService, NotesError, SavedNotes};
use deino_doc::{Doc, Flavor};
use deino_domain::{AppError, MeetingId};
use tauri::{Runtime, State};

use super::DroppedFiles;
use super::dropped::DroppedImageError;
use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        get,
        save,
        create_scheduled,
        save_image,
        attach_dropped_image,
        markdown_to_doc,
        doc_to_markdown
    ]
}

/// The meeting's notes (an empty document before the first save) and the
/// folder its image sources are relative to.
#[tauri::command]
#[specta::specta]
async fn get(
    notes: State<'_, Arc<NoteService>>,
    meeting_id: MeetingId,
) -> Result<MeetingNotes, AppError> {
    notes.get(&meeting_id).await.into_ipc()
}

/// Saves the whole document; fails with `invalid` for a document that
/// breaks the schema, is too large or points at foreign images.
#[tauri::command]
#[specta::specta]
async fn save(
    notes: State<'_, Arc<NoteService>>,
    meeting_id: MeetingId,
    doc: Doc,
) -> Result<SavedNotes, AppError> {
    notes.save(&meeting_id, doc).await.into_ipc()
}

/// Creates a `scheduled` meeting to write notes in before recording.
#[tauri::command]
#[specta::specta]
async fn create_scheduled(
    notes: State<'_, Arc<NoteService>>,
    title: String,
) -> Result<Meeting, AppError> {
    notes.create_scheduled(&title).await.into_ipc()
}

/// Stores a pasted image, sent as base64, and returns its source.
#[tauri::command]
#[specta::specta]
async fn save_image(
    notes: State<'_, Arc<NoteService>>,
    meeting_id: MeetingId,
    base64: String,
) -> Result<NoteImage, AppError> {
    let bytes = STANDARD
        .decode(base64.as_bytes())
        .map_err(|_| NotesError::UnsupportedImage);
    match bytes {
        Ok(bytes) => notes.save_image(&meeting_id, &bytes).await.into_ipc(),
        Err(error) => Err(error).into_ipc(),
    }
}

/// Stores a file dropped on the window; `path` must come from that drop.
#[tauri::command]
#[specta::specta]
async fn attach_dropped_image(
    notes: State<'_, Arc<NoteService>>,
    dropped: State<'_, Arc<DroppedFiles>>,
    meeting_id: MeetingId,
    path: PathBuf,
) -> Result<NoteImage, AppError> {
    let stored = async {
        let bytes = dropped.read(&path).await?;
        Ok::<_, DroppedImageError>(notes.save_image(&meeting_id, &bytes).await?)
    };
    stored.await.into_ipc()
}

/// Parses Markdown into a document (the summary view's input).
#[tauri::command]
#[specta::specta]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri deserializes command arguments into owned values"
)]
fn markdown_to_doc(markdown: String) -> Result<Doc, AppError> {
    deino_doc::from_markdown(&markdown)
        .map_err(NotesError::InvalidDoc)
        .into_ipc()
}

/// Writes a document as Lossless Markdown (the summary view's save).
#[tauri::command]
#[specta::specta]
fn doc_to_markdown(doc: Doc) -> Result<String, AppError> {
    let doc = doc.normalized();
    deino_doc::validate(&doc)
        .map(|()| deino_doc::to_markdown(&doc, Flavor::Lossless))
        .map_err(NotesError::InvalidDoc)
        .into_ipc()
}
