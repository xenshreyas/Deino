//! Commands of the `example` plugin.
//!
//! A command parses its input, calls one service method and maps the error.
//! It takes its service as `State<'_, Arc<Service>>`, registered by the
//! composition root, and is not generic over the Tauri runtime.
//!
//! Commands stay private and are collected in this file: `#[tauri::command]`
//! on a `pub` function exports a macro at the crate root, so two features
//! with a `list` command would collide.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::_example::{ExampleNote, ExampleService};
use deino_domain::AppError;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, for its `tauri_specta::Builder`.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![list, add]
}

/// Every note, newest first.
#[tauri::command]
#[specta::specta]
async fn list(example: State<'_, Arc<ExampleService>>) -> Result<Vec<ExampleNote>, AppError> {
    example.list().await.into_ipc()
}

/// Saves a note and returns it; fails with `invalid` for blank or overlong text.
#[tauri::command]
#[specta::specta]
async fn add(
    example: State<'_, Arc<ExampleService>>,
    text: String,
) -> Result<ExampleNote, AppError> {
    example.add(&text).await.into_ipc()
}
