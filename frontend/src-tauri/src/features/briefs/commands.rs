//! Commands of the `briefs` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::briefs::{Brief, BriefFeedback, BriefService};
use deino_domain::{AppError, CalendarEventId, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The brief service, as Tauri state.
type Briefs<'a> = State<'a, Arc<BriefService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![for_event, for_meeting, mark_read, give_feedback]
}

/// The brief of an upcoming event, or `null` when there is none.
#[tauri::command]
#[specta::specta]
async fn for_event(briefs: Briefs<'_>, event: CalendarEventId) -> Result<Option<Brief>, AppError> {
    briefs.for_event(&event).await.into_ipc()
}

/// The brief of the event a meeting note is linked to, or `null`.
#[tauri::command]
#[specta::specta]
async fn for_meeting(briefs: Briefs<'_>, meeting: MeetingId) -> Result<Option<Brief>, AppError> {
    briefs.for_meeting(&meeting).await.into_ipc()
}

/// Records that the user saw the brief; it collapses from now on.
#[tauri::command]
#[specta::specta]
async fn mark_read(briefs: Briefs<'_>, event: CalendarEventId) -> Result<(), AppError> {
    briefs.mark_read(&event).await.into_ipc()
}

/// Stores the user's verdict (the thumbs down).
#[tauri::command]
#[specta::specta]
async fn give_feedback(
    briefs: Briefs<'_>,
    event: CalendarEventId,
    feedback: BriefFeedback,
) -> Result<(), AppError> {
    briefs.give_feedback(&event, feedback).await.into_ipc()
}
