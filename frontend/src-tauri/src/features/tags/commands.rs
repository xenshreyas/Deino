//! Commands of the `tags` plugin: parse input, call one service method, map
//! the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::tags::{AppliedTag, MeetingTagLink, NewTag, Tag, TagService, TagUpdate};
use deino_domain::{AppError, MeetingId, TagId, TagSource};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The tag service, as Tauri state.
type Tags<'a> = State<'a, Arc<TagService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![list, create, update, delete, tags_of, links, apply, remove]
}

/// Every tag, by name.
#[tauri::command]
#[specta::specta]
async fn list(tags: Tags<'_>) -> Result<Vec<Tag>, AppError> {
    tags.list().await.into_ipc()
}

/// Creates a tag (the next palette colour when `color` is `null`); fails
/// with `invalid` for a blank or overlong name and `conflict` for a taken one.
#[tauri::command]
#[specta::specta]
async fn create(tags: Tags<'_>, tag: NewTag) -> Result<Tag, AppError> {
    tags.create(&tag).await.into_ipc()
}

/// Replaces the tag's name, colour, description and auto-apply switch.
#[tauri::command]
#[specta::specta]
async fn update(tags: Tags<'_>, id: TagId, update: TagUpdate) -> Result<Tag, AppError> {
    tags.update(&id, &update).await.into_ipc()
}

/// Deletes the tag; it leaves every meeting.
#[tauri::command]
#[specta::specta]
async fn delete(tags: Tags<'_>, id: TagId) -> Result<(), AppError> {
    tags.delete(&id).await.into_ipc()
}

/// The tags on the meeting, by name, with who applied them.
#[tauri::command]
#[specta::specta]
async fn tags_of(tags: Tags<'_>, meeting_id: MeetingId) -> Result<Vec<AppliedTag>, AppError> {
    tags.tags_of(&meeting_id).await.into_ipc()
}

/// Every tag on every meeting outside the trash, for the Meetings page.
#[tauri::command]
#[specta::specta]
async fn links(tags: Tags<'_>) -> Result<Vec<MeetingTagLink>, AppError> {
    tags.links().await.into_ipc()
}

/// Puts the tag on the meeting by hand (which keeps the auto-tagger away).
#[tauri::command]
#[specta::specta]
async fn apply(tags: Tags<'_>, meeting_id: MeetingId, tag_id: TagId) -> Result<(), AppError> {
    tags.apply(&meeting_id, &tag_id, TagSource::User)
        .await
        .map(|_| ())
        .into_ipc()
}

/// Takes the tag off the meeting.
#[tauri::command]
#[specta::specta]
async fn remove(tags: Tags<'_>, meeting_id: MeetingId, tag_id: TagId) -> Result<(), AppError> {
    tags.remove(&meeting_id, &tag_id).await.into_ipc()
}
