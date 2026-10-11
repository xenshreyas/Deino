//! Commands of the `insights` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::export::SavedExport;
use deino_core::insights::{
    InsightDef, InsightInput, InsightService, InsightSummary, InsightTable, InsightTemplate,
    MeetingInsights, RunCandidate, RunProgress,
};
use deino_domain::{AppError, InsightId, MeetingId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The service in Tauri state.
type Insights<'a> = State<'a, Arc<InsightService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        templates,
        list,
        get,
        create,
        update,
        delete,
        for_meeting,
        table,
        candidates,
        run,
        export_csv
    ]
}

/// The twelve ready-made insights to start from.
#[tauri::command]
#[specta::specta]
fn templates(insights: Insights<'_>) -> Vec<InsightTemplate> {
    insights.templates()
}

/// Every insight, oldest first, with where its runs stand.
#[tauri::command]
#[specta::specta]
async fn list(insights: Insights<'_>) -> Result<Vec<InsightSummary>, AppError> {
    insights.list().await.into_ipc()
}

/// One insight; `not_found` when it does not exist.
#[tauri::command]
#[specta::specta]
async fn get(insights: Insights<'_>, id: InsightId) -> Result<InsightDef, AppError> {
    insights.get(&id).await.into_ipc()
}

/// Defines an insight; `invalid` for a broken rule, `conflict` for a taken name.
#[tauri::command]
#[specta::specta]
async fn create(insights: Insights<'_>, input: InsightInput) -> Result<InsightDef, AppError> {
    insights.create(input).await.into_ipc()
}

/// Replaces an insight's definition.
#[tauri::command]
#[specta::specta]
async fn update(
    insights: Insights<'_>,
    id: InsightId,
    input: InsightInput,
) -> Result<InsightDef, AppError> {
    insights.update(&id, input).await.into_ipc()
}

/// Deletes an insight and its answers.
#[tauri::command]
#[specta::specta]
async fn delete(insights: Insights<'_>, id: InsightId) -> Result<(), AppError> {
    insights.delete(&id).await.into_ipc()
}

/// A meeting's Insights tab: its runs, and the insights it could run.
#[tauri::command]
#[specta::specta]
async fn for_meeting(
    insights: Insights<'_>,
    meeting_id: MeetingId,
) -> Result<MeetingInsights, AppError> {
    insights.for_meeting(&meeting_id).await.into_ipc()
}

/// One insight's answers across meetings.
#[tauri::command]
#[specta::specta]
async fn table(insights: Insights<'_>, id: InsightId) -> Result<InsightTable, AppError> {
    insights.table(&id).await.into_ipc()
}

/// The past meetings an insight could run on, newest first.
#[tauri::command]
#[specta::specta]
async fn candidates(insights: Insights<'_>, id: InsightId) -> Result<Vec<RunCandidate>, AppError> {
    insights.candidates(&id).await.into_ipc()
}

/// Runs an insight on the picked meetings in the background.
#[tauri::command]
#[specta::specta]
async fn run(
    insights: Insights<'_>,
    id: InsightId,
    meeting_ids: Vec<MeetingId>,
) -> Result<RunProgress, AppError> {
    insights.run(&id, &meeting_ids).await.into_ipc()
}

/// Saves an insight's table as CSV where the user picks; `null` when cancelled.
#[tauri::command]
#[specta::specta]
async fn export_csv(
    insights: Insights<'_>,
    id: InsightId,
) -> Result<Option<SavedExport>, AppError> {
    insights.export_csv(&id).await.into_ipc()
}
