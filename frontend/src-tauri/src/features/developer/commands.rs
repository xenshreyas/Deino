//! Commands of the `developer` plugin: parse input, call one service
//! method, map the error. No logic lives here. They are async so the file
//! system checks run off the main thread.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::cli_setup::{CliSetupService, CliStatus, InstallDir};
use deino_domain::AppError;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![cli_status, install_cli]
}

/// Where the bundled CLI is, where it is installed and the Claude snippets.
#[tauri::command]
#[specta::specta]
async fn cli_status(setup: State<'_, Arc<CliSetupService>>) -> Result<CliStatus, AppError> {
    Ok(setup.status())
}

/// Links `deino` in `dir` to the bundled CLI and returns the new status;
/// `conflict` when a file Deino did not make is there, `permission_denied`
/// (with the `sudo` command) when the app cannot write the folder.
#[tauri::command]
#[specta::specta]
async fn install_cli(
    setup: State<'_, Arc<CliSetupService>>,
    dir: InstallDir,
) -> Result<CliStatus, AppError> {
    setup.install(dir).into_ipc()
}
