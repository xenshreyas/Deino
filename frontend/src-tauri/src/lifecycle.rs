//! Window and run-loop event handlers: hide-to-tray on close, re-focus on
//! dock click, and resource cleanup on exit.

use tauri::{AppHandle, Manager, RunEvent, Window, WindowEvent};

use crate::{state::AppState, summary};

/// Label of the main window defined in `tauri.conf.json`.
const MAIN_WINDOW_LABEL: &str = "main";

/// Closing the main window hides it to the tray instead of quitting.
pub(crate) fn on_window_event(window: &Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if window.label() != MAIN_WINDOW_LABEL {
        return;
    }
    api.prevent_close();
    match window.hide() {
        Ok(()) => log::info!("Main window hidden to tray on close request"),
        Err(e) => log::error!("Failed to hide main window on close request: {e}"),
    }
}

/// Handles app-level run events: dock re-open (macOS) and exit cleanup.
#[allow(
    clippy::needless_pass_by_value,
    reason = "the signature Tauri requires of a run-event handler"
)]
pub(crate) fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => crate::tray::focus_main_window(app),
        RunEvent::Exit => {
            log::info!("Application exiting, cleaning up resources...");
            tauri::async_runtime::block_on(cleanup(app));
            log::info!("Application cleanup complete");
        }
        _ => {}
    }
}

/// Checkpoints the database WAL and stops the llama-helper sidecar.
async fn cleanup(app: &AppHandle) {
    match app.try_state::<AppState>() {
        Some(state) => state.db_manager.cleanup().await,
        None => log::warn!("AppState not available for database cleanup (likely first launch)"),
    }
    if let Err(e) = summary::summary_engine::force_shutdown_sidecar().await {
        log::error!("Failed to force shutdown sidecar: {e}");
    }
}
