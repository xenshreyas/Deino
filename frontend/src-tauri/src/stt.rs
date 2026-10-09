//! Speech-to-text in the app: Parakeet Unified EN 0.6B through `deino-stt`
//! (CAP-01).
//!
//! [`SttRuntime`] (Tauri state) owns the engine pool (one worker process per
//! lane plus one for batch work), the model service and the transcription
//! activity. Legacy code reaches it through the functions below, which take
//! an `AppHandle`:
//!
//! - [`start_live`] / [`finish_live`]: the live lane of a recording;
//! - [`batch_provider`]: import and retranscription;
//! - [`status`]: what `get_transcription_status` reports.
//!
//! The executable doubles as the worker process ([`worker_entry`]) and as a
//! headless benchmark ([`cli`]), both dispatched from `main` before Tauri.

pub mod cli;
mod http_source;
pub mod live;
mod provider;
mod resample;
mod runtime;
pub mod status;
pub mod worker_entry;

#[cfg(test)]
mod http_source_tests;

use std::sync::Arc;

use tauri::{AppHandle, Manager, Runtime};

pub use http_source::HttpByteSource;
pub use live::LiveAudioSink;
pub use provider::ParakeetUnifiedProvider;
pub use runtime::{SttParts, SttRuntime, SttSetupError};

use crate::audio::transcription::TranscriptionProvider;

/// The legacy transcript-config provider name of this engine.
pub const PROVIDER: &str = "parakeetUnified";

/// Dev override: a Parakeet Unified GGUF used instead of a downloaded one.
pub const MODEL_OVERRIDE_ENV: &str = "DEINO_PARAKEET_UNIFIED_MODEL";

/// What `get_transcription_status` reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Status {
    /// Units of work queued (chunks, or seconds of live audio).
    pub queued: usize,
    /// Whether transcription is running.
    pub processing: bool,
    /// Milliseconds since anything happened.
    pub idle_ms: u64,
}

fn runtime<R: Runtime>(app: &AppHandle<R>) -> Result<Arc<SttRuntime>, String> {
    app.try_state::<Arc<SttRuntime>>()
        .map(|state| Arc::clone(&state))
        .ok_or_else(|| "speech-to-text is not ready yet".to_string())
}

/// Starts the live lane for a recording with model `model` (the configured
/// file, or the recommended one when empty).
pub async fn start_live<R: Runtime>(
    app: &AppHandle<R>,
    model: &str,
) -> Result<Arc<dyn LiveAudioSink>, String> {
    let runtime = runtime(app)?;
    runtime
        .select_model(model)
        .await
        .map_err(|e| e.to_string())?;
    let sink = runtime.live_sink(app.clone());
    let live = tokio::task::spawn_blocking(move || runtime.start_live(sink))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(live)
}

/// Finishes the recording's live lane, if one runs: every fed second is
/// transcribed and the last segment emitted before this returns.
pub async fn finish_live<R: Runtime>(app: &AppHandle<R>) {
    let Ok(runtime) = runtime(app) else { return };
    if let Err(error) = tokio::task::spawn_blocking(move || runtime.finish_live()).await {
        log::error!(target: "stt", "finishing the live lane failed: {error}");
    }
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

/// What transcription is doing now.
pub fn status<R: Runtime>(app: &AppHandle<R>) -> Status {
    runtime(app)
        .map(|runtime| runtime.status())
        .unwrap_or_default()
}
