//! Wiring for speech-to-text: the model service and the engine runtime.
//!
//! Models live in `<models>/parakeet-unified/` (see [`AppPaths::models`]). Engines are
//! worker processes of this very executable. In debug builds,
//! `DEINO_PARAKEET_UNIFIED_MODEL` points the engines at a model file outside
//! the store (for development against an already-downloaded GGUF); the real
//! download path is unchanged. At startup the engine setting is healed: an
//! engine whose model is not installed falls back to an installed Parakeet
//! Unified file.

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::models::{Catalog, ModelService, ModelServiceParts, ModelStore};
use deino_core::settings::{SettingsService, healed_transcription};
use deino_stt::clock::{MonotonicClock, SystemMonotonic};
use deino_stt::{EnginePool, SupervisorConfig, WorkerLauncher};
use sysinfo::System;
use tauri::{AppHandle, Manager};

use crate::features::transcription::TauriModelEvents;
use crate::paths::AppPaths;
use crate::stt::{MODEL_OVERRIDE_ENV, NetByteSource, SttParts, SttRuntime};

/// The model store's folder under the models dir.
const MODELS_FOLDER: &str = "parakeet-unified";

/// Builds the model service and the STT runtime, manages both as
/// `Arc<…>` state, and starts the idle-unload and GPU pre-warm threads.
pub(crate) fn attach_stt(
    app: &AppHandle,
    paths: &AppPaths,
) -> Result<(), Box<dyn std::error::Error>> {
    let models = Arc::new(model_service(app, paths)?);
    let monotonic: Arc<dyn MonotonicClock> = Arc::new(SystemMonotonic::new());
    let pool = EnginePool::new(
        WorkerLauncher::current_exe()?,
        SupervisorConfig::default(),
        Arc::clone(&monotonic),
    );
    let runtime = Arc::new(SttRuntime::new(SttParts {
        pool: Arc::new(pool),
        models: Arc::clone(&models),
        monotonic,
        model_override: model_override(),
        models_dir: paths.models(),
    }));
    runtime.start_background()?;
    tauri::async_runtime::block_on(heal_engine_setting(app, &runtime));
    app.manage(models);
    app.manage(runtime);
    Ok(())
}

/// The engine self-heal, once per launch: when the configured engine's model
/// is not installed but a Parakeet Unified file is, the setting switches to
/// it (`healed_transcription`). Logs the switch once; never fatal.
async fn heal_engine_setting(app: &AppHandle, runtime: &SttRuntime) {
    let Some(settings) = app.try_state::<Arc<SettingsService>>() else {
        return log::warn!(target: "stt", "settings are not wired; the engine setting was not checked");
    };
    let current = settings.get().await.transcription;
    let Some(next) = healed_transcription(&current, &runtime.installed().await) else {
        return;
    };
    log::info!(
        target: "stt",
        "the {:?} engine's model ({:?}) is not installed; switching to Parakeet Unified ({})",
        current.engine, current.model, next.model
    );
    if let Err(error) = settings.update(|s| s.transcription = next).await {
        log::warn!(target: "stt", "could not save the healed engine setting: {error}");
    }
}

fn model_service(
    app: &AppHandle,
    paths: &AppPaths,
) -> Result<ModelService, Box<dyn std::error::Error>> {
    let root = paths.models().join(MODELS_FOLDER);
    Ok(ModelService::new(ModelServiceParts {
        catalog: Catalog::bundled()?,
        store: ModelStore::new(root),
        source: Arc::new(NetByteSource::new(super::network::model_downloads(app)?)),
        events: Arc::new(TauriModelEvents::new(app.clone())),
        memory_bytes: total_memory(),
    }))
}

/// This Mac's RAM, which picks the recommended quant.
fn total_memory() -> u64 {
    let mut system = System::new();
    system.refresh_memory();
    system.total_memory()
}

/// The development model override, honoured in debug builds only.
fn model_override() -> Option<PathBuf> {
    let path = std::env::var_os(MODEL_OVERRIDE_ENV).filter(|_| cfg!(debug_assertions))?;
    log::info!(target: "stt", "using the model override from {MODEL_OVERRIDE_ENV}");
    Some(PathBuf::from(path))
}
