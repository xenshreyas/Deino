//! Wiring for speaker diarization (CAP-03): the ONNX engine behind the
//! `diarize` stage.
//!
//! Its two models live in `<models>/diarization/` and are fetched on first
//! use through the same SHA-256-verified downloader and `ByteSource` as the
//! transcription models; Core ML keeps its compiled copy of the embedding
//! model in `<models>/diarization/coreml-cache/`.

use std::sync::Arc;

use deino_core::diarization::TrackDiarizer;
use deino_diarize::RuntimeConfig;
use deino_diarize::models::ModelStore;
use deino_diarize::track::OnnxTrackDiarizer;

use crate::paths::AppPaths;
use crate::stt::HttpByteSource;

/// The diarization models' folder under the models dir.
const MODELS_FOLDER: &str = "diarization";

/// Core ML's compiled-model cache, inside the models folder.
const ACCELERATOR_CACHE: &str = "coreml-cache";

/// The diarization engine over the app's model store and downloader.
pub(super) fn track_diarizer(
    paths: &AppPaths,
) -> Result<Arc<dyn TrackDiarizer>, Box<dyn std::error::Error>> {
    let root = paths.models().join(MODELS_FOLDER);
    let runtime = RuntimeConfig {
        accelerator_cache: Some(root.join(ACCELERATOR_CACHE)),
        ..RuntimeConfig::default()
    };
    let source = Arc::new(HttpByteSource::new()?);
    Ok(Arc::new(OnnxTrackDiarizer::new(
        ModelStore::new(root),
        source,
        runtime,
    )))
}
