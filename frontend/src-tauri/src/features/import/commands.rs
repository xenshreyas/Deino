//! Commands of the `import` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::import::{
    ImportCandidate, ImportRequest, ImportService, ImportedMeeting, MediaPicker,
};
use deino_core::watch::{FolderPicker, WatchFailure, WatchNudge, WatchService, WatchedFolder};
use deino_domain::{AppError, WatchedFileId, WatchedFolderId};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The import service, as Tauri state.
type Imports<'a> = State<'a, Arc<ImportService>>;

/// The watched-folder service, as Tauri state.
type Watch<'a> = State<'a, Arc<WatchService>>;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        pick_file,
        check,
        start,
        watched_folders,
        add_watched_folder,
        remove_watched_folder,
        set_move_to_processed,
        watch_failures,
        retry_failure,
        dismiss_failure
    ]
}

/// Opens the file dialog; the chosen recording, checked, or `null` when
/// cancelled. Fails with `invalid` for a file imports cannot read.
#[tauri::command]
#[specta::specta]
async fn pick_file(
    imports: Imports<'_>,
    picker: State<'_, Arc<dyn MediaPicker>>,
) -> Result<Option<ImportCandidate>, AppError> {
    imports.pick(picker.inner().as_ref()).await.into_ipc()
}

/// Checks a dropped file: `invalid` for a format imports do not accept or
/// one too large, `not_found` when it is gone.
#[tauri::command]
#[specta::specta]
async fn check(imports: Imports<'_>, path: String) -> Result<ImportCandidate, AppError> {
    imports.candidate(&PathBuf::from(path)).await.into_ipc()
}

/// Imports the file as a new meeting titled `title` (the file's name when
/// null) and returns it once it is processing. Converting a long file
/// takes a while; the pipeline then transcribes it.
#[tauri::command]
#[specta::specta]
async fn start(
    imports: Imports<'_>,
    path: String,
    title: Option<String>,
) -> Result<ImportedMeeting, AppError> {
    let request = ImportRequest {
        source: PathBuf::from(path),
        title,
    };
    imports.import(&request).await.into_ipc()
}

/// Every watched folder, oldest first.
#[tauri::command]
#[specta::specta]
async fn watched_folders(watch: Watch<'_>) -> Result<Vec<WatchedFolder>, AppError> {
    watch.folders().await.into_ipc()
}

/// Opens the folder dialog and watches the chosen folder; `null` when
/// cancelled, `conflict` when it is already watched.
#[tauri::command]
#[specta::specta]
async fn add_watched_folder(
    watch: Watch<'_>,
    picker: State<'_, Arc<dyn FolderPicker>>,
    nudge: State<'_, WatchNudge>,
) -> Result<Option<WatchedFolder>, AppError> {
    let added = watch.add_picked(picker.inner().as_ref()).await.into_ipc()?;
    nudge.send();
    Ok(added)
}

/// Stops watching a folder.
#[tauri::command]
#[specta::specta]
async fn remove_watched_folder(watch: Watch<'_>, id: WatchedFolderId) -> Result<(), AppError> {
    watch.remove_folder(&id).await.into_ipc()
}

/// Turns "Move to processed" on or off for a folder.
#[tauri::command]
#[specta::specta]
async fn set_move_to_processed(
    watch: Watch<'_>,
    id: WatchedFolderId,
    on: bool,
) -> Result<(), AppError> {
    watch.set_move_to_processed(&id, on).await.into_ipc()
}

/// Files a watched folder could not import, newest first.
#[tauri::command]
#[specta::specta]
async fn watch_failures(watch: Watch<'_>) -> Result<Vec<WatchFailure>, AppError> {
    watch.failures().await.into_ipc()
}

/// Imports a failed file again on the next scans.
#[tauri::command]
#[specta::specta]
async fn retry_failure(
    watch: Watch<'_>,
    nudge: State<'_, WatchNudge>,
    id: WatchedFileId,
) -> Result<(), AppError> {
    watch.retry(&id).await.into_ipc()?;
    nudge.send();
    Ok(())
}

/// Removes a failure from the list without trying again.
#[tauri::command]
#[specta::specta]
async fn dismiss_failure(watch: Watch<'_>, id: WatchedFileId) -> Result<(), AppError> {
    watch.dismiss(&id).await.into_ipc()
}
