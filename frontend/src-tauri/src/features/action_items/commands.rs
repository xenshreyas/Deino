//! Commands of the `action-items` plugin: parse input, call one service
//! method, map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::action_items::{
    ActionItemEdit, ActionItemFilter, ActionItemService, ActionItemView, FilterOptions,
    NewActionItem, SavedView, SavedViewInput, SavedViewService, TodayDigest,
};
use deino_domain::{ActionItemId, ActionItemViewId, AppError};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The items service in Tauri state.
type Items<'a> = State<'a, Arc<ActionItemService>>;
/// The saved views service in Tauri state.
type Views<'a> = State<'a, Arc<SavedViewService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        list,
        today,
        create,
        update,
        bulk_update,
        delete,
        mark_all_mine_done,
        filter_options,
        views,
        create_view,
        update_view,
        delete_view
    ]
}

/// The items `filter` matches, dated ones soonest first.
#[tauri::command]
#[specta::specta]
async fn list(items: Items<'_>, filter: ActionItemFilter) -> Result<Vec<ActionItemView>, AppError> {
    items.list(&filter).await.into_ipc()
}

/// The user's overdue and due-today open items (the "Today" digest).
#[tauri::command]
#[specta::specta]
async fn today(items: Items<'_>) -> Result<TodayDigest, AppError> {
    items.today().await.into_ipc()
}

/// Adds an item the user wrote, on a meeting or standalone.
#[tauri::command]
#[specta::specta]
async fn create(items: Items<'_>, item: NewActionItem) -> Result<ActionItemView, AppError> {
    items.create(item).await.into_ipc()
}

/// Applies one edit to one item; returns its row.
#[tauri::command]
#[specta::specta]
async fn update(
    items: Items<'_>,
    id: ActionItemId,
    edit: ActionItemEdit,
) -> Result<ActionItemView, AppError> {
    items.update(id, &edit).await.into_ipc()
}

/// Applies a status, assignee, due-day or priority edit to many items at
/// once; returns how many changed. `invalid` for a wording edit.
#[tauri::command]
#[specta::specta]
async fn bulk_update(
    items: Items<'_>,
    ids: Vec<ActionItemId>,
    edit: ActionItemEdit,
) -> Result<u32, AppError> {
    let changed = items.bulk_update(&ids, &edit).await.into_ipc()?;
    Ok(saturating_u32(changed))
}

/// Deletes the items; returns how many existed.
#[tauri::command]
#[specta::specta]
async fn delete(items: Items<'_>, ids: Vec<ActionItemId>) -> Result<u32, AppError> {
    let deleted = items.delete(&ids).await.into_ipc()?;
    Ok(saturating_u32(deleted))
}

/// Completes every open item of the user's; returns how many.
#[tauri::command]
#[specta::specta]
async fn mark_all_mine_done(items: Items<'_>) -> Result<u32, AppError> {
    let done = items.mark_all_mine_done().await.into_ipc()?;
    Ok(saturating_u32(done))
}

/// The people, companies, tags and folders to filter and assign by.
#[tauri::command]
#[specta::specta]
async fn filter_options(items: Items<'_>) -> Result<FilterOptions, AppError> {
    items.filter_options().await.into_ipc()
}

/// The saved views, in tab order.
#[tauri::command]
#[specta::specta]
async fn views(views: Views<'_>) -> Result<Vec<SavedView>, AppError> {
    views.list().await.into_ipc()
}

/// Saves a new view after the last tab; `conflict` for a name in use.
#[tauri::command]
#[specta::specta]
async fn create_view(views: Views<'_>, view: SavedViewInput) -> Result<SavedView, AppError> {
    views.create(view).await.into_ipc()
}

/// Renames a view or replaces its filter; `conflict` for a name in use.
#[tauri::command]
#[specta::specta]
async fn update_view(
    views: Views<'_>,
    id: ActionItemViewId,
    view: SavedViewInput,
) -> Result<SavedView, AppError> {
    views.update(id, view).await.into_ipc()
}

/// Deletes a saved view.
#[tauri::command]
#[specta::specta]
async fn delete_view(views: Views<'_>, id: ActionItemViewId) -> Result<(), AppError> {
    views.delete(id).await.into_ipc()
}

/// Counts cross IPC as numbers; a JS number holds any `u32`.
fn saturating_u32(count: u64) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}
