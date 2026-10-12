//! Commands of the `companies` plugin: parse input, call one service
//! method, map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::companies::Company;
use deino_core::entities::{
    CompanyFields, CompanyPage, CompanyRow, CompanyService, EntitySummarizer, LogoService,
    SummaryOutcome,
};
use deino_domain::{AppError, CompanyId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The Companies pages' service, as Tauri state.
type Companies<'a> = State<'a, Arc<CompanyService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        list,
        page,
        create,
        update,
        set_notes,
        merge,
        refresh_summary,
        fetch_logo
    ]
}

/// Every company, most recently met first.
#[tauri::command]
#[specta::specta]
async fn list(companies: Companies<'_>) -> Result<Vec<CompanyRow>, AppError> {
    companies.list().await.into_ipc()
}

/// The company page of `id` (or of the company it was merged into).
#[tauri::command]
#[specta::specta]
async fn page(companies: Companies<'_>, id: CompanyId) -> Result<CompanyPage, AppError> {
    companies.page(&id).await.into_ipc()
}

/// Adds a company; `invalid` for a blank name or bad domain, `conflict`
/// for a domain another company has.
#[tauri::command]
#[specta::specta]
async fn create(companies: Companies<'_>, fields: CompanyFields) -> Result<Company, AppError> {
    companies.create(&fields).await.into_ipc()
}

/// Saves the company form.
#[tauri::command]
#[specta::specta]
async fn update(
    companies: Companies<'_>,
    id: CompanyId,
    fields: CompanyFields,
) -> Result<Company, AppError> {
    companies.update(&id, &fields).await.into_ipc()
}

/// Saves the user's notes about a company.
#[tauri::command]
#[specta::specta]
async fn set_notes(companies: Companies<'_>, id: CompanyId, notes: String) -> Result<(), AppError> {
    companies.set_notes(&id, &notes).await.into_ipc()
}

/// Merges `absorbed` into `survivor`, moving its people; returns the survivor.
#[tauri::command]
#[specta::specta]
async fn merge(
    companies: Companies<'_>,
    survivor: CompanyId,
    absorbed: CompanyId,
) -> Result<Company, AppError> {
    companies.merge(&survivor, &absorbed).await.into_ipc()
}

/// Rewrites the company's AI summary now.
#[tauri::command]
#[specta::specta]
async fn refresh_summary(
    summaries: State<'_, Arc<EntitySummarizer>>,
    id: CompanyId,
) -> Result<SummaryOutcome, AppError> {
    summaries.summarize_company(&id).await.into_ipc()
}

/// The company's cached logo, fetched from its website first if it never
/// was (or always, with `refresh`). `permission_denied` while company logos
/// are off in Settings → Network or offline mode is on.
#[tauri::command]
#[specta::specta]
async fn fetch_logo(
    logos: State<'_, Arc<LogoService>>,
    id: CompanyId,
    refresh: bool,
) -> Result<Option<String>, AppError> {
    logos.ensure(&id, refresh).await.into_ipc()
}
