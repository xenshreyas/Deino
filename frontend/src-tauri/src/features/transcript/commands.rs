//! Commands of the `transcript` plugin: parse, call the service, map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::transcript::{Transcript, TranscriptService};
use deino_domain::{AppError, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![get]
}

/// The saved transcript of `meeting_id`: its segments in spoken order,
/// each with its speaker ID (the participants name them). An unknown meeting
/// has an empty transcript.
#[tauri::command]
#[specta::specta]
async fn get(
    transcript: State<'_, Arc<TranscriptService>>,
    meeting_id: MeetingId,
) -> Result<Transcript, AppError> {
    transcript.transcript(&meeting_id).await.into_ipc()
}
