//! Loaded models: one chat slot and one embedding slot.
//!
//! [`ModelStore`] owns the llama.cpp backend (initialised on the first load)
//! and two [`ModelSlot`]s, so embedding with a separate model never evicts
//! the chat model. A slot keeps its model while requests name the same path
//! and context size, and replaces it only once a new model has loaded, so a
//! failed load leaves the previous model usable. [`Engine`] is what the
//! generation and embedding code borrow.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::params::LlamaModelParams;

use crate::diag::diag;
use crate::gpu;

/// Threads used when the core count is unknown.
const FALLBACK_THREADS: i32 = 2;

/// Which of the two models a request uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    /// The chat model for `generate`, `stream` and `grammar`.
    Chat,
    /// The embedding model for `embed`.
    Embedding,
}

/// A model and the settings it was loaded with.
#[derive(Debug)]
pub struct LoadedModel {
    /// The llama.cpp model.
    pub model: LlamaModel,
    /// The GGUF file it came from.
    pub path: PathBuf,
    /// The context window its GPU offload was planned for.
    pub context_size: u32,
}

/// Holds at most one loaded model.
#[derive(Debug, Default)]
pub struct ModelSlot {
    loaded: Option<LoadedModel>,
}

impl ModelSlot {
    /// Whether the model at `path` is loaded for `context_size`.
    pub fn holds(&self, path: &Path, context_size: u32) -> bool {
        self.loaded
            .as_ref()
            .is_some_and(|loaded| loaded.path == path && loaded.context_size == context_size)
    }

    /// Loads the model at `path`, replacing the current one once it has loaded.
    pub fn replace(
        &mut self,
        backend: &LlamaBackend,
        path: PathBuf,
        context_size: u32,
    ) -> Result<()> {
        diag!("Loading model ({context_size} ctx)");
        let gpu_layers = gpu::default_gpu_layers(&path, context_size);
        let params = LlamaModelParams::default().with_n_gpu_layers(gpu_layers);
        let model = LlamaModel::load_from_file(backend, &path, &params)
            .with_context(|| load_error(&path))?;
        self.loaded = Some(LoadedModel {
            model,
            path,
            context_size,
        });
        diag!("Model loaded successfully");
        Ok(())
    }

    /// The loaded model, if any.
    pub fn loaded(&self) -> Option<&LoadedModel> {
        self.loaded.as_ref()
    }
}

/// The backend and both model slots.
#[derive(Debug, Default)]
pub struct ModelStore {
    backend: Option<LlamaBackend>,
    chat: ModelSlot,
    embedding: ModelSlot,
}

/// A loaded model together with the backend, ready to create contexts.
#[derive(Debug, Clone, Copy)]
pub struct Engine<'a> {
    /// The initialised llama.cpp backend.
    pub backend: &'a LlamaBackend,
    /// The model to run.
    pub loaded: &'a LoadedModel,
}

impl ModelStore {
    /// Loads a model into `kind`'s slot unless it is already there with
    /// `context_size`, initialising the backend on first use.
    pub fn load(&mut self, kind: SlotKind, path: PathBuf, context_size: u32) -> Result<()> {
        let slot = match kind {
            SlotKind::Chat => &mut self.chat,
            SlotKind::Embedding => &mut self.embedding,
        };
        if slot.holds(&path, context_size) {
            diag!("Model already loaded");
            return Ok(());
        }
        // llama.cpp asserts that the file exists in debug builds; fail cleanly.
        if !path.is_file() {
            bail!(load_error(&path));
        }
        let backend = Self::init_backend(&mut self.backend)?;
        slot.replace(backend, path, context_size)
    }

    /// The model in `kind`'s slot, or "Model not loaded".
    pub fn engine(&self, kind: SlotKind) -> Result<Engine<'_>> {
        let slot = match kind {
            SlotKind::Chat => &self.chat,
            SlotKind::Embedding => &self.embedding,
        };
        let loaded = slot.loaded().context("Model not loaded")?;
        let backend = self.backend.as_ref().context("Model not loaded")?;
        Ok(Engine { backend, loaded })
    }

    /// The backend, initialised on first use (llama.cpp allows one per process).
    fn init_backend(backend: &mut Option<LlamaBackend>) -> Result<&LlamaBackend> {
        if backend.is_none() {
            *backend = Some(LlamaBackend::init().context("Failed to init LlamaBackend")?);
        }
        backend.as_ref().context("Failed to init LlamaBackend")
    }
}

/// The load error the app has always received, with the path quoted as by `Debug`.
#[allow(
    clippy::unnecessary_debug_formatting,
    reason = "keeps the legacy `generate` error text byte for byte"
)]
fn load_error(path: &Path) -> String {
    format!("unable to load model at {path:?}")
}

/// Threads for llama.cpp: half the cores plus two, so the app's own threads
/// are never starved.
pub fn thread_count() -> i32 {
    std::thread::available_parallelism()
        .ok()
        .and_then(|cores| i32::try_from(cores.get()).ok())
        .map_or(FALLBACK_THREADS, |cores| (cores / 2 + 2).max(1))
}

/// Context parameters with a window and batch of `context_size` tokens.
pub fn context_params(context_size: u32) -> Result<LlamaContextParams> {
    let n_ctx = NonZeroU32::new(context_size).context("Invalid ctx size")?;
    let threads = thread_count();
    Ok(LlamaContextParams::default()
        .with_n_ctx(Some(n_ctx))
        .with_n_batch(context_size)
        .with_n_threads(threads)
        .with_n_threads_batch(threads))
}
