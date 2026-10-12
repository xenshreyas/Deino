//! [`SttBatchEngines`]: the batch engine the `final_transcription` stage
//! transcribes finished tracks with (CAP-08), on the engine chosen in
//! Settings.
//!
//! - **Parakeet Unified** (the default): the pool's batch worker, separate
//!   from the live lanes' workers, with token times for every word.
//! - **Parakeet TDT, Whisper** (the "Advanced" engines): the inherited
//!   engines, which give text only, so the utterance is one untimed line.
//!
//! `open` runs on the stage's blocking thread, so it may block on the async
//! settings and model calls through the runtime handle.

use std::sync::Arc;

use deino_core::final_transcription::TrackError;
use deino_core::settings::{SettingsService, TranscriptionEngine};
use deino_recording::BatchEngines;
use deino_stt::protocol::{RemoteError, RemoteErrorKind};
use deino_stt::{BatchTranscriber, BatchTranscript, EngineSlot, SttError};
use tauri::{AppHandle, Runtime};
use tokio::runtime::Handle;

use super::SttRuntime;
use crate::audio::transcription::{
    TranscriptionEngine as LegacyEngine, get_or_init_transcription_engine,
    validate_transcription_model_ready,
};

/// Opens the batch engine chosen in Settings.
pub struct SttBatchEngines<R: Runtime> {
    app: AppHandle<R>,
    stt: Arc<SttRuntime>,
    settings: Arc<SettingsService>,
    runtime: Handle,
}

impl<R: Runtime> SttBatchEngines<R> {
    /// Batch engines over the STT runtime, reading the engine from `settings`.
    pub fn new(
        app: AppHandle<R>,
        stt: Arc<SttRuntime>,
        settings: Arc<SettingsService>,
        runtime: Handle,
    ) -> Self {
        Self {
            app,
            stt,
            settings,
            runtime,
        }
    }

    fn parakeet_unified(&self, model: &str) -> Result<Arc<dyn BatchTranscriber>, TrackError> {
        self.runtime
            .block_on(self.stt.select_model(model))
            .map_err(|error| TrackError::Unavailable(Box::new(error)))?;
        let (engine, info) = self
            .stt
            .pool()
            .engine(EngineSlot::Batch)
            .map_err(|error| TrackError::Unavailable(Box::new(error)))?;
        if !info.on_gpu {
            log::warn!(target: "stt", "the after-meeting transcription runs on the CPU ({})", info.backend);
        }
        Ok(engine)
    }

    fn legacy(&self) -> Result<Arc<dyn BatchTranscriber>, TrackError> {
        let engine = self.runtime.block_on(async {
            validate_transcription_model_ready(&self.app).await?;
            get_or_init_transcription_engine(&self.app).await
        });
        let engine = engine.map_err(|message| TrackError::Unavailable(message.into()))?;
        Ok(Arc::new(LegacyBatch {
            engine,
            runtime: self.runtime.clone(),
        }))
    }
}

impl<R: Runtime> BatchEngines for SttBatchEngines<R> {
    fn open(&self) -> Result<Arc<dyn BatchTranscriber>, TrackError> {
        let transcription = self.runtime.block_on(self.settings.get()).transcription;
        match transcription.engine {
            TranscriptionEngine::ParakeetUnified => self.parakeet_unified(&transcription.model),
            TranscriptionEngine::Parakeet | TranscriptionEngine::Whisper => self.legacy(),
        }
    }
}

/// An inherited engine as a batch transcriber: text, no token times.
struct LegacyBatch {
    engine: LegacyEngine,
    runtime: Handle,
}

impl BatchTranscriber for LegacyBatch {
    fn transcribe(&self, pcm: Vec<f32>) -> Result<BatchTranscript, SttError> {
        let text = self
            .runtime
            .block_on(self.engine.transcribe_text(pcm))
            .map_err(|message| {
                SttError::Engine(RemoteError::new(RemoteErrorKind::Inference, message))
            })?;
        Ok(BatchTranscript {
            text,
            tokens: Vec::new(),
        })
    }
}
