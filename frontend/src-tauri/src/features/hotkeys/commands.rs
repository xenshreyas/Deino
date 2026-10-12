//! Commands of the `hotkeys` plugin: parse, call the service, map the error.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::hotkeys::{Hotkey, HotkeyAction, HotkeyService};
use deino_domain::AppError;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![list, set]
}

/// Every hotkey: its saved combination, default and whether it works.
#[tauri::command]
#[specta::specta]
async fn list(hotkeys: State<'_, Arc<HotkeyService>>) -> Result<Vec<Hotkey>, AppError> {
    Ok(hotkeys.list().await)
}

/// Gives `action` the combination `accelerator` (`Ctrl+Alt+Cmd+R`), or
/// turns it off with null, and returns every hotkey. Fails with `invalid`
/// for a combination Deino cannot use and `conflict` when another Deino
/// hotkey or another app has it; the previous hotkey keeps working.
#[tauri::command]
#[specta::specta]
async fn set(
    hotkeys: State<'_, Arc<HotkeyService>>,
    action: HotkeyAction,
    accelerator: Option<String>,
) -> Result<Vec<Hotkey>, AppError> {
    hotkeys.set(action, accelerator.as_deref()).await.into_ipc()
}
