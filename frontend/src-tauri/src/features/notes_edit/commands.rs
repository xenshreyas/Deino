//! Commands of the `notes-edit` plugin: parse input, call one service
//! method, map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::notes::edit::{AcceptedEdit, EditProposal, EditRequest, NoteEditService};
use deino_doc::Doc;
use deino_domain::{AppError, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The edit service, as Tauri state.
type Edits<'a> = State<'a, Arc<NoteEditService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![propose, accept]
}

/// The edit the request asks for, applied to a copy of the newest notes,
/// with the changed blocks for the preview. Nothing is stored.
#[tauri::command]
#[specta::specta]
async fn propose(edits: Edits<'_>, request: EditRequest) -> Result<EditProposal, AppError> {
    edits.propose(&request).await.into_ipc()
}

/// Stores an accepted proposal as the newest version; `conflict` when the
/// notes changed since it was made.
#[tauri::command]
#[specta::specta]
async fn accept(
    edits: Edits<'_>,
    meeting_id: MeetingId,
    base_version: u32,
    doc: Doc,
) -> Result<AcceptedEdit, AppError> {
    edits
        .accept(&meeting_id, base_version, doc)
        .await
        .into_ipc()
}
