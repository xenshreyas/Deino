//! Commands of the `comments` plugin: parse, call the service, map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::comments::{Comment, CommentService, CommentThread, NewThread};
use deino_domain::{AppError, CommentId, CommentThreadId, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![list, start, reply, edit, set_resolved, delete]
}

/// The meeting's threads, oldest first, each with its comments.
#[tauri::command]
#[specta::specta]
async fn list(
    comments: State<'_, Arc<CommentService>>,
    meeting_id: MeetingId,
) -> Result<Vec<CommentThread>, AppError> {
    comments.list(&meeting_id).await.into_ipc()
}

/// Starts a thread on a span with its first comment.
#[tauri::command]
#[specta::specta]
async fn start(
    comments: State<'_, Arc<CommentService>>,
    thread: NewThread,
) -> Result<CommentThread, AppError> {
    comments.start(thread).await.into_ipc()
}

/// Adds a reply to a thread.
#[tauri::command]
#[specta::specta]
async fn reply(
    comments: State<'_, Arc<CommentService>>,
    thread_id: CommentThreadId,
    body: String,
) -> Result<Comment, AppError> {
    comments.reply(thread_id, &body).await.into_ipc()
}

/// Replaces a comment's text.
#[tauri::command]
#[specta::specta]
async fn edit(
    comments: State<'_, Arc<CommentService>>,
    id: CommentId,
    body: String,
) -> Result<(), AppError> {
    comments.edit(id, &body).await.into_ipc()
}

/// Resolves a thread, or reopens it.
#[tauri::command]
#[specta::specta]
async fn set_resolved(
    comments: State<'_, Arc<CommentService>>,
    thread_id: CommentThreadId,
    resolved: bool,
) -> Result<(), AppError> {
    comments.set_resolved(&thread_id, resolved).await.into_ipc()
}

/// Deletes a comment; a thread goes with its last comment.
#[tauri::command]
#[specta::specta]
async fn delete(comments: State<'_, Arc<CommentService>>, id: CommentId) -> Result<(), AppError> {
    comments.delete(&id).await.into_ipc()
}
