//! Commands of the `detection` plugin: parse input, call one service method,
//! map the error. No logic lives here.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::settings::{DetectionSettings, SettingsService};
use deino_detect::{Detection, DetectionId, DetectionService};
use deino_domain::AppError;
use serde::Serialize;
use tauri::{Runtime, State};

use crate::error::IntoIpc;

/// The plugin's commands, collected here so they can stay private.
pub(super) fn collect<R: Runtime>() -> tauri_specta::Commands<R> {
    tauri_specta::collect_commands![current, take_notes, dismiss, ignore_app, settings]
}

/// The detection settings, and what this Mac supports.
#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DetectionOverview {
    /// The saved settings (changed through `plugin:settings|patch`).
    pub settings: DetectionSettings,
    /// Meeting detection works on this platform (macOS).
    pub supported: bool,
    /// Accessibility is granted, so browser meetings are recognised by URL
    /// within seconds; without it they prompt after a sustained hold.
    pub reads_browser_urls: bool,
}

/// The prompt on screen, for a prompt window that opens after the event.
#[tauri::command]
#[specta::specta]
async fn current(
    detection: State<'_, Arc<DetectionService>>,
) -> Result<Option<Detection>, AppError> {
    Ok(detection.current())
}

/// "Take notes": closes the prompt and starts recording its meeting. Fails
/// with `not_found` when the prompt already closed.
#[tauri::command]
#[specta::specta]
async fn take_notes(
    detection: State<'_, Arc<DetectionService>>,
    id: DetectionId,
) -> Result<(), AppError> {
    detection.take_notes(id).into_ipc()
}

/// "Ignore" or "Don't record": closes the prompt; the app stays quiet for
/// ten minutes. Fails with `not_found` when the prompt already closed.
#[tauri::command]
#[specta::specta]
async fn dismiss(
    detection: State<'_, Arc<DetectionService>>,
    id: DetectionId,
) -> Result<(), AppError> {
    detection.dismiss(id).into_ipc()
}

/// "Always ignore this app": closes the prompt and adds the app to
/// `detection.ignoredApps`. Fails with `not_found` when the prompt already closed.
#[tauri::command]
#[specta::specta]
async fn ignore_app(
    detection: State<'_, Arc<DetectionService>>,
    id: DetectionId,
) -> Result<(), AppError> {
    detection.ignore_app(id).await.into_ipc()
}

/// The detection settings, and whether this Mac supports detection and
/// browser URLs.
#[tauri::command]
#[specta::specta]
async fn settings(
    settings: State<'_, Arc<SettingsService>>,
) -> Result<DetectionOverview, AppError> {
    Ok(DetectionOverview {
        settings: settings.get().await.detection,
        supported: deino_detect::is_supported(),
        reads_browser_urls: deino_detect::reads_browser_urls(),
    })
}
