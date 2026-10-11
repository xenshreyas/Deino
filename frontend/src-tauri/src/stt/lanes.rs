//! [`SttLanes`]: the recorder's `LaneEngines`, over the configured engine.
//!
//! - **Parakeet Unified** (the default): each lane streams on its own
//!   worker from the engine pool (`deino_recording::engine::StreamingLane`).
//! - **Parakeet TDT, Whisper** (the "Advanced" engines): they cannot stream,
//!   so each lane transcribes the voice spans its detector closes
//!   (`UtteranceLane`) through the inherited engines.
//!
//! `open` is called from the recorder's blocking start-up thread, so it may
//! block on the async settings and model calls through the runtime handle.

use std::sync::Arc;

use deino_core::settings::{SettingsService, TranscriptionEngine};
use deino_recording::Lane;
use deino_recording::engine::{
    EngineError, LaneEngines, LaneTranscriber, UtteranceLane, UtteranceTranscriber,
};
use deino_stt::SessionSink;
use tauri::{AppHandle, Runtime};
use tokio::runtime::Handle;

use super::SttRuntime;
use crate::audio::transcription::{
    TranscriptionEngine as LegacyEngine, get_or_init_transcription_engine,
    validate_transcription_model_ready,
};

/// Opens each lane on the engine chosen in Settings.
pub struct SttLanes<R: Runtime> {
    app: AppHandle<R>,
    stt: Arc<SttRuntime>,
    settings: Arc<SettingsService>,
    runtime: Handle,
}

impl<R: Runtime> SttLanes<R> {
    /// Lanes over the STT runtime, reading the engine from `settings`.
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

    fn streaming(
        &self,
        lane: Lane,
        model: &str,
        sink: Box<dyn SessionSink>,
    ) -> Result<Box<dyn LaneTranscriber>, EngineError> {
        self.runtime
            .block_on(self.stt.select_model(model))
            .map_err(|source| failed(lane, source))?;
        Ok(Box::new(self.stt.open_lane(lane, sink)?))
    }

    fn utterances(
        &self,
        lane: Lane,
        sink: Box<dyn SessionSink>,
    ) -> Result<Box<dyn LaneTranscriber>, EngineError> {
        let engine = self.runtime.block_on(async {
            validate_transcription_model_ready(&self.app).await?;
            get_or_init_transcription_engine(&self.app).await
        });
        let engine = engine.map_err(|message| failed(lane, std::io::Error::other(message)))?;
        let transcriber = Arc::new(LegacyUtterances {
            engine,
            runtime: self.runtime.clone(),
        });
        Ok(Box::new(UtteranceLane::start(lane, transcriber, sink)?))
    }
}

impl<R: Runtime> LaneEngines for SttLanes<R> {
    fn open(
        &self,
        lane: Lane,
        sink: Box<dyn SessionSink>,
    ) -> Result<Box<dyn LaneTranscriber>, EngineError> {
        let transcription = self.runtime.block_on(self.settings.get()).transcription;
        match transcription.engine {
            TranscriptionEngine::ParakeetUnified => {
                self.streaming(lane, &transcription.model, sink)
            }
            TranscriptionEngine::Parakeet | TranscriptionEngine::Whisper => {
                self.utterances(lane, sink)
            }
        }
    }
}

/// An inherited batch engine as an utterance transcriber.
struct LegacyUtterances {
    engine: LegacyEngine,
    runtime: Handle,
}

impl UtteranceTranscriber for LegacyUtterances {
    fn transcribe(
        &self,
        pcm: Vec<f32>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.runtime
            .block_on(self.engine.transcribe_text(pcm))
            .map_err(|message| {
                Box::new(std::io::Error::other(message)) as Box<dyn std::error::Error + Send + Sync>
            })
    }
}

fn failed(lane: Lane, source: impl std::error::Error + Send + Sync + 'static) -> EngineError {
    EngineError {
        lane,
        source: Box::new(source),
    }
}
