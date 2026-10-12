//! Commands of the `importers` plugin: parse input, call one service
//! method, map the error. `start` hands the claimed job to the runtime.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::importers::{ImportJobs, ImportPreview, ImportProgress, Importer};
use deino_core::watch::FolderPicker;
use deino_domain::{AppError, ImportSource};
use deino_importers::ImportSources;
use deino_secrets::SecretStatus;
use serde::{Deserialize, Serialize};
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The job, as Tauri state.
type Jobs<'a> = State<'a, Arc<ImportJobs>>;
/// The sources' keys and adapters, as Tauri state.
type Sources<'a> = State<'a, Arc<ImportSources>>;

/// What to import from: a provider, or a folder with its path.
#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportTarget {
    /// The source.
    pub source: ImportSource,
    /// The folder, for `folder`; ignored for a provider.
    pub folder: Option<String>,
}

/// A source and whether it is ready to import from.
#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportSourceStatus {
    /// The source.
    pub source: ImportSource,
    /// Its API key: `stored` once saved (always for a folder).
    pub key: SecretStatus,
}

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![
        sources,
        save_key,
        forget_key,
        pick_folder,
        preview,
        start,
        cancel,
        status
    ]
}

/// Every source, in the order Settings → Import lists them.
#[tauri::command]
#[specta::specta]
async fn sources(sources: Sources<'_>) -> Result<Vec<ImportSourceStatus>, AppError> {
    let mut statuses = Vec::with_capacity(ImportSource::ALL.len());
    for &source in ImportSource::ALL {
        let key = sources.key_status(source).await.into_ipc()?;
        statuses.push(ImportSourceStatus { source, key });
    }
    Ok(statuses)
}

/// Saves `key` for `source` in the Keychain and switches its service on.
/// Fails with `invalid` for a blank key.
#[tauri::command]
#[specta::specta]
async fn save_key(sources: Sources<'_>, source: ImportSource, key: String) -> Result<(), AppError> {
    sources.save_key(source, &key).await.into_ipc()
}

/// Forgets `source`'s key and switches its service off.
#[tauri::command]
#[specta::specta]
async fn forget_key(sources: Sources<'_>, source: ImportSource) -> Result<(), AppError> {
    sources.forget_key(source).await.into_ipc()
}

/// Opens the folder dialog; the chosen folder, or `null` when cancelled.
#[tauri::command]
#[specta::specta]
async fn pick_folder(picker: State<'_, Arc<dyn FolderPicker>>) -> Result<Option<String>, AppError> {
    let folder = picker.pick_folder().await;
    Ok(folder.map(|path| path.to_string_lossy().into_owned()))
}

/// Counts what importing `target` would add: `not_found` without a key,
/// `permission_denied` for a refused key or in offline mode.
#[tauri::command]
#[specta::specta]
async fn preview(
    jobs: Jobs<'_>,
    sources: Sources<'_>,
    target: ImportTarget,
) -> Result<ImportPreview, AppError> {
    let importer = resolve(&sources, &target).await?;
    jobs.preview(importer.as_ref()).await.into_ipc()
}

/// Starts importing `target` in the background and returns the first
/// progress; `conflict` while another import runs.
#[tauri::command]
#[specta::specta]
async fn start(
    jobs: Jobs<'_>,
    sources: Sources<'_>,
    target: ImportTarget,
) -> Result<ImportProgress, AppError> {
    let importer = resolve(&sources, &target).await?;
    let token = jobs.begin(target.source).into_ipc()?;
    let jobs = Arc::clone(&jobs);
    let first = jobs.status();
    // Owned by the job slot: it ends on its own or when cancelled.
    tauri::async_runtime::spawn(async move {
        jobs.run(importer.as_ref(), token).await;
    });
    Ok(first)
}

/// Stops the running import after the meeting it is on; false when none runs.
#[tauri::command]
#[specta::specta]
async fn cancel(jobs: Jobs<'_>) -> Result<bool, AppError> {
    Ok(jobs.cancel())
}

/// The latest progress (`idle` before any import).
#[tauri::command]
#[specta::specta]
async fn status(jobs: Jobs<'_>) -> Result<ImportProgress, AppError> {
    Ok(jobs.status())
}

/// The adapter for `target`.
async fn resolve(
    sources: &ImportSources,
    target: &ImportTarget,
) -> Result<Arc<dyn Importer>, AppError> {
    let folder = target.folder.as_deref().map(PathBuf::from);
    sources.importer(target.source, folder).await.into_ipc()
}
