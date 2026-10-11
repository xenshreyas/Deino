//! Speech-to-text in the app: Parakeet Unified EN 0.6B through `deino-stt`
//! (CAP-01), and the recorder's lanes over it (CAP-02).
//!
//! [`SttRuntime`] (Tauri state) owns the engine pool (one worker process per
//! live lane plus one for batch work) and the model service. [`SttLanes`]
//! opens a recording's Me and Them lanes on the engine chosen in Settings.
//! Legacy code reaches the runtime through [`batch_provider`] (import and
//! retranscription) and [`validate`].
//!
//! The executable doubles as the worker process ([`worker_entry`]) and as a
//! headless benchmark ([`cli`]), both dispatched from `main` before Tauri.

pub mod cli;
mod http_source;
pub mod lanes;
mod provider;
mod runtime;
pub mod settings;
pub mod worker_entry;

#[cfg(test)]
mod http_source_tests;

use std::sync::Arc;

use tauri::{AppHandle, Manager, Runtime};

pub use http_source::NetByteSource;
pub use lanes::SttLanes;
pub use provider::ParakeetUnifiedProvider;
pub use runtime::{SttParts, SttRuntime, SttSetupError};

use crate::audio::transcription::TranscriptionProvider;

/// The legacy transcript-config provider name of this engine.
pub const PROVIDER: &str = "parakeetUnified";

/// Dev override: a Parakeet Unified GGUF used instead of a downloaded one.
pub const MODEL_OVERRIDE_ENV: &str = "DEINO_PARAKEET_UNIFIED_MODEL";

fn runtime<R: Runtime>(app: &AppHandle<R>) -> Result<Arc<SttRuntime>, String> {
    app.try_state::<Arc<SttRuntime>>()
        .map(|state| Arc::clone(&state))
        .ok_or_else(|| "speech-to-text is not ready yet".to_string())
}

/// Checks that model `model` is downloaded and points the engines at it.
pub async fn validate<R: Runtime>(app: &AppHandle<R>, model: &str) -> Result<(), String> {
    runtime(app)?
        .select_model(model)
        .await
        .map(drop)
        .map_err(|e| e.to_string())
}

/// The batch provider for import and retranscription with model `model`.
pub async fn batch_provider<R: Runtime>(
    app: &AppHandle<R>,
    model: &str,
) -> Result<Arc<dyn TranscriptionProvider>, String> {
    let runtime = runtime(app)?;
    let id = runtime
        .select_model(model)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Arc::new(ParakeetUnifiedProvider::new(runtime.pool(), id)))
}
