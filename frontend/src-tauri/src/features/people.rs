//! `people` feature: the IPC surface of `deino_core::people`, exposed as the
//! inline plugin `plugin:people|<command>`.
//!
//! NOT-01 adds `search`, which feeds the notepad's @mention suggestions;
//! ORG-04 adds the people pages' commands here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

#[cfg(test)]
mod contract_tests;

use std::sync::Arc;

use deino_core::people::{PeopleService, Person};
use deino_domain::AppError;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// Plugin name; must match this feature's entry in `features::manifest`.
pub const NAME: &str = "people";

/// The plugin's commands, for the invoke handler and the bindings.
pub fn specta<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::new()
        .plugin_name(NAME)
        .commands(tauri_specta::collect_commands![search])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Builds the `people` plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(NAME)
        .invoke_handler(specta().invoke_handler())
        .build()
}

/// Up to eight people whose name, a word of it, or email starts with `query`;
/// the first people by name for an empty query.
#[tauri::command]
#[specta::specta]
async fn search(
    people: State<'_, Arc<PeopleService>>,
    query: String,
) -> Result<Vec<Person>, AppError> {
    people.search(&query).await.into_ipc()
}
