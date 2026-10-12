//! Commands of the `meetings` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::meetings::{
    BulkOutcome, Meeting, MeetingBulkService, MeetingCursor, MeetingFilter, MeetingListService,
    MeetingMergeService, MeetingPage, MeetingService, MeetingView, MeetingViewInput,
    MeetingViewService, MergeOutcome, TrashOutcome, TrashService, TrashedMeeting,
};
use deino_domain::{AppError, FolderId, MeetingId, MeetingViewId, TagId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The list service in Tauri state.
type List<'a> = State<'a, Arc<MeetingListService>>;
/// The saved views service in Tauri state.
type Views<'a> = State<'a, Arc<MeetingViewService>>;
/// The trash in Tauri state.
type Trash<'a> = State<'a, Arc<TrashService>>;
/// The bulk actions in Tauri state.
type Bulk<'a> = State<'a, Arc<MeetingBulkService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        list,
        get,
        rename,
        page,
        matching_ids,
        views,
        create_view,
        update_view,
        delete_view,
        trash,
        restore,
        trashed,
        delete_forever,
        tag_many,
        untag_many,
        file_many,
        merge
    ]
}

/// Every meeting, newest first.
#[tauri::command]
#[specta::specta]
async fn list(meetings: State<'_, Arc<MeetingService>>) -> Result<Vec<Meeting>, AppError> {
    meetings.list().await.into_ipc()
}

/// The meeting with `id`; fails with `not_found` if there is none.
#[tauri::command]
#[specta::specta]
async fn get(meetings: State<'_, Arc<MeetingService>>, id: MeetingId) -> Result<Meeting, AppError> {
    meetings.get(&id).await.into_ipc()
}

/// Renames the meeting and returns it; fails with `invalid` for a blank or
/// overlong title and `not_found` for an unknown `id`.
#[tauri::command]
#[specta::specta]
async fn rename(
    meetings: State<'_, Arc<MeetingService>>,
    id: MeetingId,
    title: String,
) -> Result<Meeting, AppError> {
    meetings.rename(&id, &title).await.into_ipc()
}

/// Up to `size` meetings `filter` matches, newest first, after `after`;
/// `invalid` for the audience filter without the user's address.
#[tauri::command]
#[specta::specta]
async fn page(
    list: List<'_>,
    filter: MeetingFilter,
    after: Option<MeetingCursor>,
    size: u32,
) -> Result<MeetingPage, AppError> {
    list.page(&filter, after.as_ref(), size).await.into_ipc()
}

/// Every meeting `filter` matches, newest first (⌘A).
#[tauri::command]
#[specta::specta]
async fn matching_ids(list: List<'_>, filter: MeetingFilter) -> Result<Vec<MeetingId>, AppError> {
    list.matching_ids(&filter).await.into_ipc()
}

/// The saved views, in tab order.
#[tauri::command]
#[specta::specta]
async fn views(views: Views<'_>) -> Result<Vec<MeetingView>, AppError> {
    views.list().await.into_ipc()
}

/// Saves a view after the last tab; `conflict` for a taken name.
#[tauri::command]
#[specta::specta]
async fn create_view(views: Views<'_>, view: MeetingViewInput) -> Result<MeetingView, AppError> {
    views.create(view).await.into_ipc()
}

/// Renames a view or replaces its filter.
#[tauri::command]
#[specta::specta]
async fn update_view(
    views: Views<'_>,
    id: MeetingViewId,
    view: MeetingViewInput,
) -> Result<MeetingView, AppError> {
    views.update(id, view).await.into_ipc()
}

/// Deletes a view.
#[tauri::command]
#[specta::specta]
async fn delete_view(views: Views<'_>, id: MeetingViewId) -> Result<(), AppError> {
    views.delete(id).await.into_ipc()
}

/// Moves meetings to the trash.
#[tauri::command]
#[specta::specta]
async fn trash(trash: Trash<'_>, ids: Vec<MeetingId>) -> Result<TrashOutcome, AppError> {
    trash.trash(&ids).await.into_ipc()
}

/// Takes meetings out of the trash, whole.
#[tauri::command]
#[specta::specta]
async fn restore(trash: Trash<'_>, ids: Vec<MeetingId>) -> Result<TrashOutcome, AppError> {
    trash.restore(&ids).await.into_ipc()
}

/// The trash, most recently trashed first.
#[tauri::command]
#[specta::specta]
async fn trashed(trash: Trash<'_>) -> Result<Vec<TrashedMeeting>, AppError> {
    trash.list().await.into_ipc()
}

/// Deletes trashed meetings for good, with their recordings.
#[tauri::command]
#[specta::specta]
async fn delete_forever(trash: Trash<'_>, ids: Vec<MeetingId>) -> Result<TrashOutcome, AppError> {
    trash.delete_forever(&ids).await.into_ipc()
}

/// Puts a tag on every meeting in `ids`.
#[tauri::command]
#[specta::specta]
async fn tag_many(
    bulk: Bulk<'_>,
    ids: Vec<MeetingId>,
    tag: TagId,
) -> Result<BulkOutcome, AppError> {
    bulk.tag(&ids, &tag).await.into_ipc()
}

/// Takes a tag off every meeting in `ids`.
#[tauri::command]
#[specta::specta]
async fn untag_many(
    bulk: Bulk<'_>,
    ids: Vec<MeetingId>,
    tag: TagId,
) -> Result<BulkOutcome, AppError> {
    bulk.untag(&ids, &tag).await.into_ipc()
}

/// Files every meeting in `ids` into a folder.
#[tauri::command]
#[specta::specta]
async fn file_many(
    bulk: Bulk<'_>,
    ids: Vec<MeetingId>,
    folder: FolderId,
) -> Result<BulkOutcome, AppError> {
    bulk.file(&ids, &folder).await.into_ipc()
}

/// Merges two or more meetings into the earliest of them.
#[tauri::command]
#[specta::specta]
async fn merge(
    merger: State<'_, Arc<MeetingMergeService>>,
    ids: Vec<MeetingId>,
) -> Result<MergeOutcome, AppError> {
    merger.merge(&ids).await.into_ipc()
}
