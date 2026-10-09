//! One-time application startup, run from the Tauri `setup` hook.
//!
//! Each step is a small function so startup order is readable in [`run`].
//! Settings and secrets come first, because everything else reads them; slow
//! work (engine warm-up, model manager, notifications) is spawned so the
//! window appears immediately; the database open blocks, because every
//! legacy command expects [`crate::state::AppState`] to exist.

use tauri::{App, AppHandle, Manager, Wry};

use crate::notifications::commands::{NotificationManagerState, initialize_notification_manager};
use crate::paths::AppPaths;
use crate::{composition, database, onnx_runtime, parakeet_engine, summary, tray, whisper_engine};

/// Error type accepted by Tauri's setup hook.
type SetupResult = Result<(), Box<dyn std::error::Error>>;

/// Tauri setup hook: initializes runtime services in dependency order.
pub(crate) fn run(app: &mut App) -> SetupResult {
    let handle = app.handle();
    let paths = AppPaths::resolve(handle)?;
    tauri::async_runtime::block_on(composition::attach_settings(handle, &paths))?;
    onnx_runtime::init_bundled(handle);
    create_tray(handle);
    spawn_notification_manager(handle.clone());
    init_transcription_engines(&paths);
    spawn_summary_model_manager(handle.clone());
    tauri::async_runtime::block_on(database::setup::initialize_database_on_startup(handle))?;
    #[cfg(debug_assertions)]
    tauri::async_runtime::block_on(composition::attach_example(handle))?;
    configure_template_dirs(&paths);
    log::info!("Application setup complete");
    Ok(())
}

fn create_tray(app: &AppHandle) {
    if let Err(e) = tray::create_tray(app) {
        log::error!("Failed to create system tray: {e}");
    }
}

/// Creates the notification manager with consent and permission defaults.
fn spawn_notification_manager(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let manager = match initialize_notification_manager(app.clone()).await {
            Ok(manager) => manager,
            Err(e) => return log::error!("Failed to initialize notification manager: {e}"),
        };
        if let Err(e) = manager.set_consent(true).await {
            log::error!("Failed to set initial notification consent: {e}");
        }
        if let Err(e) = manager.request_permission().await {
            log::error!("Failed to request initial notification permission: {e}");
        }
        let state = app.state::<NotificationManagerState<Wry>>();
        *state.write().await = Some(manager);
        log::info!("Notification system initialized");
    });
}

/// Points both STT engines at the models dir and warms them up in the background.
fn init_transcription_engines(paths: &AppPaths) {
    whisper_engine::commands::set_models_directory(paths.models());
    parakeet_engine::commands::set_models_directory(paths.models());
    tauri::async_runtime::spawn(async {
        if let Err(e) = whisper_engine::commands::whisper_init().await {
            log::error!("Failed to initialize Whisper engine on startup: {e}");
        }
    });
    tauri::async_runtime::spawn(async {
        if let Err(e) = parakeet_engine::commands::parakeet_init().await {
            log::error!("Failed to initialize Parakeet engine on startup: {e}");
        }
    });
}

/// Initializes the built-in summary model manager; it is lazily retried on first use.
fn spawn_summary_model_manager(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let result = summary::summary_engine::commands::init_model_manager_at_startup(&app).await;
        match result {
            Ok(()) => log::info!("ModelManager initialized at startup"),
            Err(e) => log::warn!("ModelManager startup init failed, will retry lazily: {e}"),
        }
    });
}

/// Registers the bundled (resources) and user (app data) template directories.
fn configure_template_dirs(paths: &AppPaths) {
    match paths.bundled_templates() {
        Some(bundled) => summary::templates::set_bundled_templates_dir(bundled),
        None => log::warn!("No resource directory; only the built-in templates are available"),
    }
    summary::templates::set_custom_templates_dir(paths.custom_templates());
}
