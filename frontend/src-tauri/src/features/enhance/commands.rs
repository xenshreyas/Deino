//! Commands of the `enhance` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::enhance::{
    CitedSegment, EnhanceService, EnhancedNotesView, Overwrite, SavedEnhancedNotes, VersionInfo,
};
use deino_doc::Doc;
use deino_domain::{AppError, MeetingId, SegmentId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The enhanced-notes service in Tauri state.
type Notes<'a> = State<'a, Arc<EnhanceService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![get, versions, enhance, save, restore, cited]
}

/// Version `version` of the meeting's notes (the newest when null), with
/// its regenerate reasons; null before the first version.
#[tauri::command]
#[specta::specta]
async fn get(
    notes: Notes<'_>,
    meeting_id: MeetingId,
    version: Option<u32>,
) -> Result<Option<EnhancedNotesView>, AppError> {
    notes.view(&meeting_id, version).await.into_ipc()
}

/// Every version of the meeting's notes, newest first.
#[tauri::command]
#[specta::specta]
async fn versions(notes: Notes<'_>, meeting_id: MeetingId) -> Result<Vec<VersionInfo>, AppError> {
    notes.versions(&meeting_id).await.into_ipc()
}

/// Writes the meeting's notes as a new version. Without `replaceEdits`, a
/// newest version the user edited fails with `conflict` so the UI can ask.
#[tauri::command]
#[specta::specta]
async fn enhance(
    notes: Notes<'_>,
    meeting_id: MeetingId,
    replace_edits: bool,
) -> Result<EnhancedNotesView, AppError> {
    let overwrite = if replace_edits {
        Overwrite::ReplaceEdits
    } else {
        Overwrite::KeepEdits
    };
    notes.enhance(&meeting_id, overwrite).await.into_ipc()
}

/// Saves the user's edit of the newest version.
#[tauri::command]
#[specta::specta]
async fn save(
    notes: Notes<'_>,
    meeting_id: MeetingId,
    version: u32,
    doc: Doc,
) -> Result<SavedEnhancedNotes, AppError> {
    notes.save(&meeting_id, version, doc).await.into_ipc()
}

/// Makes a copy of an earlier version the newest one.
#[tauri::command]
#[specta::specta]
async fn restore(
    notes: Notes<'_>,
    meeting_id: MeetingId,
    version: u32,
) -> Result<EnhancedNotesView, AppError> {
    notes.restore(&meeting_id, version).await.into_ipc()
}

/// The transcript excerpts behind a block's citations, in time order.
#[tauri::command]
#[specta::specta]
async fn cited(
    notes: Notes<'_>,
    meeting_id: MeetingId,
    segment_ids: Vec<SegmentId>,
) -> Result<Vec<CitedSegment>, AppError> {
    notes.cited(&meeting_id, &segment_ids).await.into_ipc()
}
