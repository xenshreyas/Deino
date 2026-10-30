//! Wiring for speaker diarization (CAP-03): the ONNX engine behind the
//! `diarize` stage.
//!
//! Its two models ship inside the app (`speakers/` among its resources). A
//! build without them (a checkout without Git LFS) falls back to
//! `<models>/diarization/`, fetched on first use through `deino-core`'s
//! SHA-256-verified downloader over `deino-net`'s `hf_models` service. The
//! stage downloads on its own, so it uses the ordinary scoped client, not
//! `approved_download`: offline mode and the user's network switches block
//! it, and the stage then skips speakers rather than holding up notes. Core
//! ML keeps its compiled copy of the embedding model in
//! `<models>/diarization/coreml-cache/`.

use std::sync::Arc;

use deino_core::diarization::{NoWordAligner, TrackDiarizer, WordAligner};
use deino_diarize::RuntimeConfig;
use deino_diarize::align::OnnxWordAligner;
use deino_diarize::models::{ModelStore, bundled};
use deino_diarize::python::PythonEngine;
use deino_diarize::track::OnnxTrackDiarizer;
use deino_net::{Net, ServiceId};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::paths::AppPaths;
use crate::stt::NetByteSource;

/// The diarization models' folder under the models dir.
const MODELS_FOLDER: &str = "diarization";

/// Core ML's compiled-model cache, inside the models folder.
const ACCELERATOR_CACHE: &str = "coreml-cache";

/// The owner's opt-in Python speaker engine's folder under the data dir
/// (`scripts/setup-python-speakers.sh`); deleting it turns the engine off.
const PYTHON_ENGINE_FOLDER: &str = "python-diarization";

/// The diarizer and the word aligner the `diarize` stage uses.
pub(super) type SpeakerEngines = (Arc<dyn TrackDiarizer>, Arc<dyn WordAligner>);

/// The Python engine for both when it is installed, else the ONNX engine
/// and the bundled wav2vec2 word aligner.
pub(super) fn speaker_engines(app: &AppHandle) -> Result<SpeakerEngines, String> {
    let paths = AppPaths::resolve(app).map_err(|error| error.to_string())?;
    if let Some(engine) = PythonEngine::installed(&paths.data().join(PYTHON_ENGINE_FOLDER)) {
        log::info!(target: "pipeline", "speakers: using the Python engine (Community-1 and word alignment)");
        let engine = Arc::new(engine);
        return Ok((engine.clone(), engine));
    }
    Ok((track_diarizer(app)?, word_aligner(app)))
}

/// The diarization engine over the app's model store and network client.
fn track_diarizer(app: &AppHandle) -> Result<Arc<dyn TrackDiarizer>, String> {
    let paths = AppPaths::resolve(app).map_err(|error| error.to_string())?;
    let net = app
        .try_state::<Net>()
        .ok_or("the network client is not ready yet")?;
    let source = Arc::new(NetByteSource::new(Arc::new(
        net.scoped(ServiceId::HfModels),
    )));
    let root = paths.models().join(MODELS_FOLDER);
    let runtime = RuntimeConfig {
        accelerator_cache: Some(root.join(ACCELERATOR_CACHE)),
        ..RuntimeConfig::default()
    };
    let store = bundled_store(app).unwrap_or_else(|| ModelStore::new(root));
    Ok(Arc::new(OnnxTrackDiarizer::new(store, source, runtime)))
}

/// The bundled wav2vec2 aligner, or none when the app lacks it (a checkout
/// without Git LFS): lines then stay untimed, as before.
fn word_aligner(app: &AppHandle) -> Arc<dyn WordAligner> {
    let model = app
        .path()
        .resolve(bundled::FOLDER, BaseDirectory::Resource)
        .map(|folder| folder.join(bundled::ALIGNER.file_name));
    match model {
        Ok(model) if file_size(&model) == Some(bundled::ALIGNER.bytes) => {
            Arc::new(OnnxWordAligner::new(model, RuntimeConfig::default()))
        }
        _ => {
            log::warn!(target: "pipeline", "speakers: the bundled word aligner is missing; lines stay untimed");
            Arc::new(NoWordAligner)
        }
    }
}

fn file_size(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|metadata| metadata.len())
}

/// The models inside the app, when both are there at their pinned sizes.
fn bundled_store(app: &AppHandle) -> Option<ModelStore> {
    let folder = app
        .path()
        .resolve(bundled::FOLDER, BaseDirectory::Resource)
        .ok()?;
    let store = ModelStore::new(folder);
    if store.verified_paths().is_ok() {
        return Some(store);
    }
    log::warn!(target: "pipeline", "speakers: the bundled models are missing; downloading instead");
    None
}
