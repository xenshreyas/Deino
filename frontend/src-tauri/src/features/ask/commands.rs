//! Commands of the `ask` plugin: parse input, call one service method, map
//! the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::ask::{AskMessage, AskRequest, AskService};
use deino_domain::{AppError, MeetingId, RecipeId};
use tauri::ipc::Channel;
use tauri::{Runtime, State};

use super::events::{AskStreamEvent, ChannelSink};
use crate::error::IntoIpc;

/// The ask service, as Tauri state.
type Ask<'a> = State<'a, Arc<AskService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![thread, ask, preview, clear]
}

/// The meeting's thread, oldest first.
#[tauri::command]
#[specta::specta]
async fn thread(ask: Ask<'_>, meeting: MeetingId) -> Result<Vec<AskMessage>, AppError> {
    ask.thread(&meeting).await.into_ipc()
}

/// Answers a question or runs a recipe on the meeting, streaming the answer
/// to `on_event`; resolves with the saved answer and its citations.
#[tauri::command]
#[specta::specta]
async fn ask(
    ask: Ask<'_>,
    meeting: MeetingId,
    request: AskRequest,
    on_event: Channel<AskStreamEvent>,
) -> Result<AskMessage, AppError> {
    ask.ask(&meeting, request, &ChannelSink(on_event))
        .await
        .into_ipc()
}

/// Runs a recipe on the meeting without saving anything (Recipes → Preview).
#[tauri::command]
#[specta::specta]
async fn preview(
    ask: Ask<'_>,
    meeting: MeetingId,
    recipe: RecipeId,
    on_event: Channel<AskStreamEvent>,
) -> Result<AskMessage, AppError> {
    ask.preview(&meeting, &recipe, &ChannelSink(on_event))
        .await
        .into_ipc()
}

/// Empties the meeting's thread.
#[tauri::command]
#[specta::specta]
async fn clear(ask: Ask<'_>, meeting: MeetingId) -> Result<(), AppError> {
    ask.clear(&meeting).await.into_ipc()
}
