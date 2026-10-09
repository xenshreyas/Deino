//! Commands of the debug-only `dev` plugin.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_domain::AppError;
use deino_testkit::fixture::{FixtureSeeder, SeedReport};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![seed_fixtures]
}

/// Writes the fixture meetings into the database, replacing earlier fixture
/// rows and leaving every other meeting alone, and returns what it wrote.
#[tauri::command]
#[specta::specta]
async fn seed_fixtures(seeder: State<'_, Arc<FixtureSeeder>>) -> Result<SeedReport, AppError> {
    seeder.seed().await.into_ipc()
}
