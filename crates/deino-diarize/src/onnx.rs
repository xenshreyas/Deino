//! ONNX Runtime sessions for the two models.
//!
//! Adapted from anarlog `crates/onnx/src/lib.rs` (MIT): sessions are built
//! from a file path (anarlog embeds the models with `include_bytes!`), the
//! intra-op thread count comes from [`RuntimeConfig`] instead of a global,
//! and the Core ML cache lives where the caller says.
//!
//! Core ML runs the embedding model as a `NeuralNetwork` on CPU + Neural
//! Engine: `MLProgram` cannot bound the model's variable frame count and
//! returns non-finite values, and the GPU unit fails to resize dynamically.
//! This combination matched the CPU output bit for bit and ran about ten
//! times faster in the spike. Any accelerator failure falls back to the CPU,
//! which produces the same values, only slower.

use std::path::Path;

use ort::session::Session;
use ort::session::builder::{GraphOptimizationLevel, SessionBuilder};

use crate::config::RuntimeConfig;

/// A CPU session for the model at `path`.
pub(crate) fn cpu_session(path: &Path, runtime: &RuntimeConfig) -> Result<Session, ort::Error> {
    builder(runtime)?.commit_from_file(path)
}

/// A session for the model at `path` on the accelerator when `runtime` asks
/// for one and the platform has it, else on the CPU.
pub(crate) fn accelerated_session(
    path: &Path,
    runtime: &RuntimeConfig,
) -> Result<Session, ort::Error> {
    if !runtime.accelerate_embedding {
        return cpu_session(path, runtime);
    }
    match coreml::session(path, runtime) {
        Ok(session) => Ok(session),
        Err(error) => {
            log::warn!(target: "deino_diarize", "accelerated session unavailable, using the CPU: {error}");
            cpu_session(path, runtime)
        }
    }
}

/// The shared builder: fixed threads, one inter-op thread, full graph
/// optimisation.
fn builder(runtime: &RuntimeConfig) -> Result<SessionBuilder, ort::Error> {
    Session::builder()?
        .with_intra_threads(runtime.intra_threads.get())?
        .with_inter_threads(1)?
        .with_optimization_level(GraphOptimizationLevel::Level3)
}

#[cfg(target_os = "macos")]
mod coreml {
    use std::path::Path;

    use ort::execution_providers::CoreMLExecutionProvider;
    use ort::execution_providers::coreml::{CoreMLComputeUnits, CoreMLModelFormat};
    use ort::session::Session;

    use super::builder;
    use crate::config::RuntimeConfig;

    /// A Core ML session, rebuilding a stale compiled-model cache once: a
    /// cache a crash left half-written would otherwise pin every later load
    /// to the CPU.
    pub(super) fn session(path: &Path, runtime: &RuntimeConfig) -> Result<Session, ort::Error> {
        match commit(path, runtime) {
            Err(error) if clear_cache(runtime) => {
                log::warn!(target: "deino_diarize", "rebuilt the Core ML cache after: {error}");
                commit(path, runtime)
            }
            result => result,
        }
    }

    fn commit(path: &Path, runtime: &RuntimeConfig) -> Result<Session, ort::Error> {
        let mut provider = CoreMLExecutionProvider::default()
            .with_model_format(CoreMLModelFormat::NeuralNetwork)
            .with_compute_units(CoreMLComputeUnits::CPUAndNeuralEngine);
        if let Some(cache) = &runtime.accelerator_cache {
            provider = provider.with_model_cache_dir(cache.display());
        }
        builder(runtime)?
            .with_execution_providers([provider.build()])?
            .commit_from_file(path)
    }

    /// Removes the compiled-model cache; whether there was one.
    fn clear_cache(runtime: &RuntimeConfig) -> bool {
        runtime
            .accelerator_cache
            .as_ref()
            .is_some_and(|cache| std::fs::remove_dir_all(cache).is_ok())
    }
}

#[cfg(not(target_os = "macos"))]
mod coreml {
    use std::path::Path;

    use ort::session::Session;

    use crate::config::RuntimeConfig;

    /// No accelerator off macOS: the CPU session.
    pub(super) fn session(path: &Path, runtime: &RuntimeConfig) -> Result<Session, ort::Error> {
        super::cpu_session(path, runtime)
    }
}
