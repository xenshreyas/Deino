//! The process-wide llama-helper sidecar and model-path lookups.
//!
//! Generation itself goes through the LLM gateway (FND-09): the app's
//! `llm::local` adapter resolves a model here, gets the sidecar from
//! [`sidecar`], and streams an exchange. This module keeps the one sidecar
//! manager the app runs and its shutdown paths.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, PoisonError, RwLock};

use anyhow::{Result, anyhow};
use tokio::sync::Mutex;

use super::models;
use super::sidecar::SidecarManager;

/// The one sidecar manager, created on first use.
static SIDECAR_MANAGER: LazyLock<Mutex<Option<Arc<SidecarManager>>>> =
    LazyLock::new(|| Mutex::new(None));

/// Resolved model paths, so a request does not hit the filesystem each time.
static MODEL_PATH_CACHE: LazyLock<RwLock<HashMap<String, PathBuf>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// The sidecar manager, created on first use.
pub async fn sidecar(app_data_dir: &Path) -> Result<Arc<SidecarManager>> {
    let mut global = SIDECAR_MANAGER.lock().await;
    if let Some(manager) = global.as_ref() {
        return Ok(Arc::clone(manager));
    }
    log::info!("Initializing sidecar manager");
    let manager = Arc::new(SidecarManager::new(app_data_dir.to_path_buf())?);
    *global = Some(Arc::clone(&manager));
    Ok(manager)
}

/// The downloaded GGUF file of `model_name`; fails when it is unknown or
/// not downloaded yet.
pub fn cached_model_path(app_data_dir: &Path, model_name: &str) -> Result<PathBuf> {
    let cached = MODEL_PATH_CACHE
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .get(model_name)
        .cloned();
    if let Some(path) = cached.filter(|path| path.exists()) {
        return Ok(path);
    }
    let model_path = models::get_model_path(&app_data_dir.to_path_buf(), model_name)?;
    if !model_path.exists() {
        return Err(anyhow!(
            "the built-in model '{}' is not downloaded; download it in Settings",
            model_name
        ));
    }
    MODEL_PATH_CACHE
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(model_name.to_string(), model_path.clone());
    Ok(model_path)
}

/// Shutdown the global sidecar (graceful cleanup)
/// Detaches the current manager and spawns a background task to drain active requests
pub async fn shutdown_sidecar_gracefully() -> Result<()> {
    let manager_opt = SIDECAR_MANAGER.lock().await.take();
    if let Some(manager) = manager_opt {
        log::info!("Detaching sidecar manager for graceful shutdown");
        tauri::async_runtime::spawn(async move {
            if let Err(e) = manager.shutdown_gracefully().await {
                log::error!("Error during graceful shutdown: {}", e);
            }
        });
    }
    Ok(())
}

/// Force shutdown the global sidecar (for app exit)
/// Directly kills the process without waiting for active requests to complete.
pub async fn force_shutdown_sidecar() -> Result<()> {
    let manager_opt = SIDECAR_MANAGER.lock().await.take();
    if let Some(manager) = manager_opt {
        log::info!("Force shutting down sidecar for app exit");
        manager.shutdown().await?;
    }
    Ok(())
}

/// Check if sidecar is healthy
pub async fn is_sidecar_healthy() -> bool {
    match SIDECAR_MANAGER.lock().await.as_ref() {
        Some(manager) => manager.is_healthy(),
        None => false,
    }
}
