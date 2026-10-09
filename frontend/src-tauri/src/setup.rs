//! One-time application startup, run from the Tauri `setup` hook.
//!
//! Each step is a small function so startup order is readable in [`run`].
//! Slow work (engine warm-up, model manager, notifications) is spawned so the
//! window appears immediately; only the database open blocks, because every
//! legacy command expects [`crate::state::AppState`] to exist.

use tauri::{App, AppHandle, Manager, Wry};

use crate::notifications::commands::{initialize_notification_manager, NotificationManagerState};
use crate::{database, onnx_runtime, parakeet_engine, summary, tray, whisper_engine};

/// Error type accepted by Tauri's setup hook.
type SetupResult = Result<(), Box<dyn std::error::Error>>;

/// Tauri setup hook: initializes runtime services in dependency order.
pub(crate) fn run(app: &mut App) -> SetupResult {
    let handle = app.handle();
    onnx_runtime::init_bundled(handle);
    create_tray(handle);
    spawn_notification_manager(handle.clone());
    init_transcription_engines(handle);
    spawn_summary_model_manager(handle.clone());
    tauri::async_runtime::block_on(database::setup::initialize_database_on_startup(handle))?;
    configure_template_dirs(handle);
    log::info!("Application setup complete");
    Ok(())
}

fn create_tray(app: &AppHandle) {
    if let Err(e) = tray::create_tray(app) {
        log::error!("Failed to create system tray: {}", e);
    }
}

/// Creates the notification manager with consent and permission defaults.
fn spawn_notification_manager(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let manager = match initialize_notification_manager(app.clone()).await {
            Ok(manager) => manager,
            Err(e) => return log::error!("Failed to initialize notification manager: {}", e),
        };
        if let Err(e) = manager.set_consent(true).await {
            log::error!("Failed to set initial notification consent: {}", e);
        }
        if let Err(e) = manager.request_permission().await {
            log::error!("Failed to request initial notification permission: {}", e);
        }
        let state = app.state::<NotificationManagerState<Wry>>();
        *state.write().await = Some(manager);
        log::info!("Notification system initialized");
    });
}

/// Points both STT engines at the app data dir and warms them up in the background.
fn init_transcription_engines(app: &AppHandle) {
    whisper_engine::commands::set_models_directory(app);
    parakeet_engine::commands::set_models_directory(app);
    tauri::async_runtime::spawn(async {
        if let Err(e) = whisper_engine::commands::whisper_init().await {
            log::error!("Failed to initialize Whisper engine on startup: {}", e);
        }
    });
    tauri::async_runtime::spawn(async {
        if let Err(e) = parakeet_engine::commands::parakeet_init().await {
            log::error!("Failed to initialize Parakeet engine on startup: {}", e);
        }
    });
}

/// Initializes the built-in summary model manager; it is lazily retried on first use.
fn spawn_summary_model_manager(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let result = summary::summary_engine::commands::init_model_manager_at_startup(&app).await;
        match result {
            Ok(_) => log::info!("ModelManager initialized at startup"),
            Err(e) => log::warn!("ModelManager startup init failed, will retry lazily: {}", e),
        }
    });
}

/// Registers the bundled (resources) and user (app data) template directories.
fn configure_template_dirs(app: &AppHandle) {
    match app.path().resource_dir() {
        Ok(resources) => summary::templates::set_bundled_templates_dir(resources.join("templates")),
        Err(e) => log::warn!("Failed to resolve resource directory for templates: {}", e),
    }
    match app.path().app_data_dir() {
        Ok(app_data) => summary::templates::set_custom_templates_dir(app_data.join("templates")),
        Err(e) => log::warn!("Failed to resolve app data directory for templates: {}", e),
    }
}
