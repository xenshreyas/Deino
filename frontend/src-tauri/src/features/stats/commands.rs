//! Commands of the `stats` plugin.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::stats::{
    Heatmap, RecapService, RecapStatus, StatsOverview, StatsRange, StatsService, YearRecap,
};
use deino_domain::AppError;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, for its `tauri_specta::Builder`.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![overview, heatmap, recap_status, generate_recap]
}

/// The numbers of `range`, ending now.
#[tauri::command]
#[specta::specta]
async fn overview(
    stats: State<'_, Arc<StatsService>>,
    range: StatsRange,
) -> Result<StatsOverview, AppError> {
    stats.overview(range).await.into_ipc()
}

/// Meetings per day over the last year.
#[tauri::command]
#[specta::specta]
async fn heatmap(stats: State<'_, Arc<StatsService>>) -> Result<Heatmap, AppError> {
    stats.heatmap().await.into_ipc()
}

/// This year's recap, if generated, and whether it can be.
#[tauri::command]
#[specta::specta]
async fn recap_status(recap: State<'_, Arc<RecapService>>) -> Result<RecapStatus, AppError> {
    recap.status().await.into_ipc()
}

/// This year's recap, written now if it was not yet; fails with `conflict`
/// outside December and `unavailable` without a model.
#[tauri::command]
#[specta::specta]
async fn generate_recap(recap: State<'_, Arc<RecapService>>) -> Result<YearRecap, AppError> {
    recap.generate().await.into_ipc()
}
