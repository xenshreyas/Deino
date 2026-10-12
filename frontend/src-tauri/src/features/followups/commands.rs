//! Commands of the `followups` plugin: parse input, call one service
//! method, map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::followups::{FollowUpDraft, FollowUpEdit, FollowUpService, FollowUpSummary};
use deino_domain::{AppError, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The follow-up service, as Tauri state.
type FollowUps<'a> = State<'a, Arc<FollowUpService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![suggested, for_meeting, save, dismiss, open_in_mail, copy]
}

/// Home's "Suggested follow-ups": ready drafts of the last three days.
#[tauri::command]
#[specta::specta]
async fn suggested(follow_ups: FollowUps<'_>) -> Result<Vec<FollowUpSummary>, AppError> {
    follow_ups.suggested().await.into_ipc()
}

/// The meeting's ready draft, or `null`.
#[tauri::command]
#[specta::specta]
async fn for_meeting(
    follow_ups: FollowUps<'_>,
    meeting: MeetingId,
) -> Result<Option<FollowUpDraft>, AppError> {
    follow_ups.for_meeting(&meeting).await.into_ipc()
}

/// Stores the user's edit of the draft and returns it.
#[tauri::command]
#[specta::specta]
async fn save(
    follow_ups: FollowUps<'_>,
    meeting: MeetingId,
    edit: FollowUpEdit,
) -> Result<FollowUpDraft, AppError> {
    follow_ups.save(&meeting, edit).await.into_ipc()
}

/// Dismisses the meeting's follow-up for good.
#[tauri::command]
#[specta::specta]
async fn dismiss(follow_ups: FollowUps<'_>, meeting: MeetingId) -> Result<(), AppError> {
    follow_ups.dismiss(&meeting).await.into_ipc()
}

/// Saves the edit and opens it in the user's mail app.
#[tauri::command]
#[specta::specta]
async fn open_in_mail(
    follow_ups: FollowUps<'_>,
    meeting: MeetingId,
    edit: FollowUpEdit,
) -> Result<(), AppError> {
    follow_ups.open_in_mail(&meeting, edit).await.into_ipc()
}

/// Saves the edit and copies its body to the clipboard.
#[tauri::command]
#[specta::specta]
async fn copy(
    follow_ups: FollowUps<'_>,
    meeting: MeetingId,
    edit: FollowUpEdit,
) -> Result<(), AppError> {
    follow_ups.copy(&meeting, edit).await.into_ipc()
}
