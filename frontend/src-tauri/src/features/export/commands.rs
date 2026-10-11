//! Commands of the `export` plugin. None takes a path: files and folders
//! come from dialogs the Rust side opens.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::export::{
    AutoExportService, CopyKind, ExportPreview, ExportService, FolderDialog, FullExportService,
    FullExportStatus, SaveDialog, SavedExport,
};
use deino_domain::{AppError, MeetingId};
use deino_export::{ExportFormat, ExportSections};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, for its `tauri_specta::Builder`.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        preview,
        save,
        copy,
        email,
        choose_folder,
        start_full_export,
        cancel_full_export,
        full_export_status
    ]
}

/// What exporting the meeting as `format` with `sections` would write.
#[tauri::command]
#[specta::specta]
async fn preview(
    exports: State<'_, Arc<ExportService>>,
    meeting_id: MeetingId,
    format: ExportFormat,
    sections: ExportSections,
) -> Result<ExportPreview, AppError> {
    exports
        .preview(&meeting_id, format, sections)
        .await
        .into_ipc()
}

/// Opens the save dialog and writes the meeting there; `null` when the user
/// cancelled.
#[tauri::command]
#[specta::specta]
async fn save(
    exports: State<'_, Arc<ExportService>>,
    meeting_id: MeetingId,
    format: ExportFormat,
    sections: ExportSections,
) -> Result<Option<SavedExport>, AppError> {
    exports.save(&meeting_id, format, sections).await.into_ipc()
}

/// Copies the meeting as rich text, Markdown or its transcript.
#[tauri::command]
#[specta::specta]
async fn copy(
    exports: State<'_, Arc<ExportService>>,
    meeting_id: MeetingId,
    kind: CopyKind,
) -> Result<(), AppError> {
    exports.copy(&meeting_id, kind).await.into_ipc()
}

/// Opens an email draft of the notes in the mail app.
#[tauri::command]
#[specta::specta]
async fn email(
    exports: State<'_, Arc<ExportService>>,
    meeting_id: MeetingId,
) -> Result<(), AppError> {
    exports.email(&meeting_id).await.into_ipc()
}

/// Asks for the auto-export folder and saves it in the settings; returns
/// the folder, or `null` when the user cancelled.
#[tauri::command]
#[specta::specta]
async fn choose_folder(
    auto: State<'_, Arc<AutoExportService>>,
    dialog: State<'_, Arc<dyn FolderDialog>>,
) -> Result<Option<String>, AppError> {
    let settings = auto.choose_folder(dialog.as_ref()).await.into_ipc()?;
    Ok(settings.and_then(|settings| settings.export.folder))
}

/// Asks where to save the account export and starts it in the background;
/// `false` when the user cancelled. Progress arrives as `FullExportProgress`.
#[tauri::command]
#[specta::specta]
async fn start_full_export(
    full: State<'_, Arc<FullExportService>>,
    dialog: State<'_, Arc<dyn SaveDialog>>,
    include_audio: bool,
) -> Result<bool, AppError> {
    full.start(dialog.as_ref(), include_audio).await.into_ipc()
}

/// Cancels the running account export; its partial archive is deleted.
#[tauri::command]
#[specta::specta]
async fn cancel_full_export(full: State<'_, Arc<FullExportService>>) -> Result<(), AppError> {
    full.cancel();
    Ok(())
}

/// Where the account export is (for a page opened mid-export).
#[tauri::command]
#[specta::specta]
async fn full_export_status(
    full: State<'_, Arc<FullExportService>>,
) -> Result<FullExportStatus, AppError> {
    Ok(full.status())
}
