//! Wiring for speaker diarization (CAP-03): the ONNX engine behind the
//! `diarize` stage.
//!
//! Its two models live in `<models>/diarization/` and are fetched on first
//! use through `deino-core`'s SHA-256-verified downloader over `deino-net`'s
//! `hf_models` service. The stage downloads on its own, so it uses the
//! ordinary scoped client, not `approved_download`: offline mode and the
//! user's network switches block it, and the stage then skips speakers
//! rather than holding up notes. Core ML keeps its compiled copy of the
//! embedding model in `<models>/diarization/coreml-cache/`.

use std::sync::Arc;

use deino_core::diarization::{NoWordAligner, TrackDiarizer, WordAligner};
use deino_diarize::RuntimeConfig;
use deino_diarize::models::ModelStore;
use deino_diarize::python::PythonEngine;
use deino_diarize::track::OnnxTrackDiarizer;
use deino_net::{Net, ServiceId};
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
/// and no aligner.
pub(super) fn speaker_engines(app: &AppHandle) -> Result<SpeakerEngines, String> {
    let paths = AppPaths::resolve(app).map_err(|error| error.to_string())?;
    if let Some(engine) = PythonEngine::installed(&paths.data().join(PYTHON_ENGINE_FOLDER)) {
        log::info!(target: "pipeline", "speakers: using the Python engine (Community-1 and word alignment)");
        let engine = Arc::new(engine);
        return Ok((engine.clone(), engine));
    }
    Ok((track_diarizer(app)?, Arc::new(NoWordAligner)))
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
    Ok(Arc::new(OnnxTrackDiarizer::new(
        ModelStore::new(root),
        source,
        runtime,
    )))
}
