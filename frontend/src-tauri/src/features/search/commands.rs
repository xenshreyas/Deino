//! Commands of the `search` plugin: parse input, call one service method,
//! map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::search::semantic::{IndexStatus, SemanticAdmin};
use deino_core::search::{
    FilterChip, FilterKind, ReindexReport, SearchRequest, SearchResults, SearchService,
};
use deino_domain::AppError;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        search,
        suggest_filters,
        reindex_all,
        index_status,
        download_model,
        cancel_download,
        delete_model,
        start_backfill
    ]
}

/// Meetings, moments, people, companies, action items and folders matching
/// the request, best first in each section, with the filters that applied.
#[tauri::command]
#[specta::specta]
async fn search(
    search: State<'_, Arc<SearchService>>,
    request: SearchRequest,
) -> Result<SearchResults, AppError> {
    search.search(&request).await.into_ipc()
}

/// `@` filter candidates whose name starts with `prefix`, of `kind` if given.
#[tauri::command]
#[specta::specta]
async fn suggest_filters(
    search: State<'_, Arc<SearchService>>,
    prefix: String,
    kind: Option<FilterKind>,
) -> Result<Vec<FilterChip>, AppError> {
    search.suggest_filters(&prefix, kind).await.into_ipc()
}

/// Rebuilds the keyword index from every source.
#[tauri::command]
#[specta::specta]
async fn reindex_all(search: State<'_, Arc<SearchService>>) -> Result<ReindexReport, AppError> {
    search.reindex_all().await.into_ipc()
}

/// The embedding model, how many meetings it covers and the backfill.
#[tauri::command]
#[specta::specta]
async fn index_status(admin: State<'_, Arc<SemanticAdmin>>) -> Result<IndexStatus, AppError> {
    admin.status().await.into_ipc()
}

/// Downloads and verifies the embedding model, then starts the backfill.
/// Resolves when the download ends; fails with `cancelled` or `network`.
#[tauri::command]
#[specta::specta]
async fn download_model(admin: State<'_, Arc<SemanticAdmin>>) -> Result<IndexStatus, AppError> {
    admin.download_model().await.into_ipc()
}

/// Stops the model's download, keeping what arrived; false if none ran.
#[tauri::command]
#[specta::specta]
async fn cancel_download(admin: State<'_, Arc<SemanticAdmin>>) -> Result<bool, AppError> {
    admin.cancel_download().into_ipc()
}

/// Deletes the model file (search becomes keyword-only).
#[tauri::command]
#[specta::specta]
async fn delete_model(admin: State<'_, Arc<SemanticAdmin>>) -> Result<IndexStatus, AppError> {
    admin.delete_model().await.into_ipc()
}

/// Embeds every finished meeting not embedded yet (a no-op while running).
#[tauri::command]
#[specta::specta]
async fn start_backfill(admin: State<'_, Arc<SemanticAdmin>>) -> Result<IndexStatus, AppError> {
    admin.start_backfill().await.into_ipc()
}
