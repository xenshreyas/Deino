//! What the benchmark runs the stages with: the legacy Whisper engine as
//! the app's `LegacyBatch` runs it, and stand-ins for the recorder, the
//! switch and the downloads.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use deino_core::final_transcription::TrackError;
use deino_core::models::{ByteSource, ByteStream, FetchError};
use deino_domain::MeetingId;
use deino_recording::{BatchEngines, FollowWanted, Recorder};
use deino_stt::protocol::{RemoteError, RemoteErrorKind};
use deino_stt::{BatchTranscriber, BatchTranscript, SttError};
use tokio::runtime::Handle;

use crate::whisper_engine::WhisperEngine;

/// The legacy Whisper engine, loaded once, as the app's `LegacyBatch` runs it.
pub(super) struct WhisperEngines {
    pub(super) models: PathBuf,
    pub(super) model: String,
    pub(super) runtime: Handle,
    pub(super) engine: std::sync::Mutex<Option<Arc<dyn BatchTranscriber>>>,
}

impl BatchEngines for WhisperEngines {
    fn open(&self) -> Result<Arc<dyn BatchTranscriber>, TrackError> {
        let unavailable = |message: String| TrackError::Unavailable(message.into());
        let mut engine = self.engine.lock().map_err(|e| unavailable(e.to_string()))?;
        if let Some(engine) = engine.as_ref() {
            return Ok(Arc::clone(engine));
        }
        let whisper = WhisperEngine::new_with_models_dir(Some(self.models.clone()))
            .map_err(|e| unavailable(e.to_string()))?;
        self.runtime
            .block_on(async {
                whisper.discover_models().await?;
                whisper.load_model(&self.model).await
            })
            .map_err(|e| unavailable(e.to_string()))?;
        let loaded: Arc<dyn BatchTranscriber> = Arc::new(Legacy {
            whisper: Arc::new(whisper),
            runtime: self.runtime.clone(),
        });
        *engine = Some(Arc::clone(&loaded));
        Ok(loaded)
    }

    fn model(&self) -> Option<String> {
        Some(format!("Whisper/{}", self.model))
    }
}

struct Legacy {
    whisper: Arc<WhisperEngine>,
    runtime: Handle,
}

impl BatchTranscriber for Legacy {
    fn transcribe(&self, pcm: Vec<f32>) -> Result<BatchTranscript, SttError> {
        let text = self
            .runtime
            .block_on(self.whisper.transcribe_audio_with_confidence(pcm))
            .map(|(text, _, _)| text)
            .map_err(|e| {
                SttError::Engine(RemoteError::new(RemoteErrorKind::Inference, e.to_string()))
            })?;
        Ok(BatchTranscript {
            text,
            tokens: Vec::new(),
        })
    }
}

/// The bench drives its one pass itself.
pub(super) struct NothingRecording;

impl Recorder for NothingRecording {
    fn recording(&self) -> Option<MeetingId> {
        None
    }
}

pub(super) struct Always;

#[async_trait]
impl FollowWanted for Always {
    async fn wanted(&self) -> bool {
        true
    }
}

/// The models are bundled; nothing is downloaded.
pub(super) struct NoDownloads;

#[async_trait]
impl ByteSource for NoDownloads {
    async fn open(&self, _url: &str, _offset: u64) -> Result<Box<dyn ByteStream>, FetchError> {
        Err(FetchError::Stalled)
    }
}
