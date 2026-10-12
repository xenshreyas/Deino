//! Commands of the `privacy` plugin: the app lock and the library's status.
//! The guard always admits them, so the lock screen keeps working.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::lock::{LockService, LockState};
use deino_domain::AppError;
use tauri::{Runtime, State};

use super::library::{LibraryStatus, LibraryStatusCell};
use crate::error::IntoIpc;

/// What macOS shows under "Deino is trying to …" in the unlock prompt.
const UNLOCK_REASON: &str = "unlock your meeting notes";

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![state, touch, lock, unlock, library]
}

/// Whether the app is locked now (an overdue idle lock takes effect here).
#[tauri::command]
#[specta::specta]
async fn state(service: State<'_, Arc<LockService>>) -> Result<LockState, AppError> {
    Ok(service.state())
}

/// The owner used the window: restarts the idle timer. The UI sends it at
/// most every few seconds; it never unlocks.
#[tauri::command]
#[specta::specta]
async fn touch(service: State<'_, Arc<LockService>>) -> Result<LockState, AppError> {
    Ok(service.activity())
}

/// Locks now when the app lock is on; otherwise stays unlocked.
#[tauri::command]
#[specta::specta]
async fn lock(service: State<'_, Arc<LockService>>) -> Result<LockState, AppError> {
    Ok(service.lock_now())
}

/// Asks for Touch ID (or the Mac's password) and unlocks. Fails with
/// `cancelled` when the prompt is dismissed, `permission_denied` when it
/// does not match, and `unavailable` when this Mac cannot authenticate.
#[tauri::command]
#[specta::specta]
async fn unlock(service: State<'_, Arc<LockService>>) -> Result<LockState, AppError> {
    service.unlock(UNLOCK_REASON).await.into_ipc()
}

/// Where opening the library stands: converting, ready or unavailable.
#[tauri::command]
#[specta::specta]
async fn library(status: State<'_, Arc<LibraryStatusCell>>) -> Result<LibraryStatus, AppError> {
    Ok(status.get())
}
