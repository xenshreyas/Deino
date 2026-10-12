//! Commands of the `links` plugin.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::links::{LinkService, Navigation};
use deino_domain::AppError;
use tauri::{Runtime, State};

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![take]
}

/// The navigations links and hotkeys queued, oldest first; taking them
/// empties the queue. The app lock's guard refuses this while Deino is
/// locked, and the service hands out nothing then either.
#[tauri::command]
#[specta::specta]
async fn take(links: State<'_, Arc<LinkService>>) -> Result<Vec<Navigation>, AppError> {
    Ok(links.take())
}
