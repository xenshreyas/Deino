//! Commands of the `vocabulary` plugin: parse input, call one service
//! method, map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::vocabulary::{
    CorrectedSegment, TermInput, UndoReport, UndoScope, VocabTerm, VocabularyService,
};
use deino_domain::{AppError, MeetingId, SegmentId, VocabTermId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The vocabulary service, as Tauri state.
type Vocabulary<'a> = State<'a, Arc<VocabularyService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![list, create, update, delete, corrections, undo]
}

/// Every entry: replacements, then terms, each by spelling.
#[tauri::command]
#[specta::specta]
async fn list(vocabulary: Vocabulary<'_>) -> Result<Vec<VocabTerm>, AppError> {
    vocabulary.list().await.into_ipc()
}

/// Adds a term or replacement; `invalid` for a blank or overlong entry,
/// `conflict` when one of the same kind is already spelled so.
#[tauri::command]
#[specta::specta]
async fn create(vocabulary: Vocabulary<'_>, input: TermInput) -> Result<VocabTerm, AppError> {
    vocabulary.create(&input).await.into_ipc()
}

/// Replaces an entry's kind, spelling and sounds-like spellings.
#[tauri::command]
#[specta::specta]
async fn update(
    vocabulary: Vocabulary<'_>,
    id: VocabTermId,
    input: TermInput,
) -> Result<VocabTerm, AppError> {
    vocabulary.update(&id, &input).await.into_ipc()
}

/// Deletes an entry; corrections it made stay until undone.
#[tauri::command]
#[specta::specta]
async fn delete(vocabulary: Vocabulary<'_>, id: VocabTermId) -> Result<(), AppError> {
    vocabulary.delete(&id).await.into_ipc()
}

/// The meeting's corrections that are not undone, in transcript order.
#[tauri::command]
#[specta::specta]
async fn corrections(
    vocabulary: Vocabulary<'_>,
    meeting_id: MeetingId,
) -> Result<Vec<CorrectedSegment>, AppError> {
    vocabulary.corrections(&meeting_id).await.into_ipc()
}

/// Undoes one segment's correction, or every correction of the meeting;
/// segments edited since keep their edit.
#[tauri::command]
#[specta::specta]
async fn undo(
    vocabulary: Vocabulary<'_>,
    meeting_id: MeetingId,
    segment_id: Option<SegmentId>,
) -> Result<UndoReport, AppError> {
    let scope = segment_id
        .as_ref()
        .map_or(UndoScope::Meeting(&meeting_id), UndoScope::Segment);
    vocabulary.undo(scope).await.into_ipc()
}
