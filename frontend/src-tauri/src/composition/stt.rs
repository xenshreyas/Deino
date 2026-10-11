//! Wiring for speech-to-text: the model service and the engine runtime.
//!
//! Models live in `<models>/parakeet-unified/` (see [`AppPaths::models`]). Engines are
//! worker processes of this very executable. In debug builds,
//! `DEINO_PARAKEET_UNIFIED_MODEL` points the engines at a model file outside
//! the store (for development against an already-downloaded GGUF); the real
//! download path is unchanged.

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::models::{Catalog, ModelService, ModelServiceParts, ModelStore};
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
    }));
    runtime.start_background()?;
    app.manage(models);
    app.manage(runtime);
    Ok(())
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
