//! Parakeet Unified behind the legacy `TranscriptionProvider` trait.
//!
//! Import, retranscription and the legacy chunk worker transcribe one VAD
//! segment (25 s at most) at a time through this provider. It runs on the
//! pool's batch engine, so it never interrupts a live stream, and token
//! confidences become the segment's confidence.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::models::ModelFileId;
use deino_stt::{BatchTranscript, EnginePool, EngineSlot, SttError};

use crate::audio::transcription::{TranscriptResult, TranscriptionError, TranscriptionProvider};

/// Confidence reported when the model gives none.
const DEFAULT_CONFIDENCE: f32 = 0.85;

/// Batch transcription with Parakeet Unified EN 0.6B.
pub struct ParakeetUnifiedProvider {
    pool: Arc<EnginePool>,
    model: ModelFileId,
}

impl ParakeetUnifiedProvider {
    /// A provider over the pool's batch engine, which runs `model`.
    pub fn new(pool: Arc<EnginePool>, model: ModelFileId) -> Self {
        Self { pool, model }
    }

    fn run(pool: &EnginePool, audio: Vec<f32>) -> Result<BatchTranscript, SttError> {
        let (engine, info) = pool.engine(EngineSlot::Batch)?;
        if !info.on_gpu {
            log::warn!(target: "stt", "batch transcription is running on the CPU ({})", info.backend);
        }
        engine.transcribe(audio)
    }
}

/// The mean token confidence, when the model reports any.
#[allow(
    clippy::cast_precision_loss,
    reason = "a segment has a few hundred tokens, far below f32's exact range"
)]
fn confidence(transcript: &BatchTranscript) -> Option<f32> {
    let scores: Vec<f32> = transcript
        .tokens
        .iter()
        .filter_map(|t| t.confidence)
        .collect();
    (!scores.is_empty()).then(|| scores.iter().sum::<f32>() / scores.len() as f32)
}

fn failed(error: impl std::fmt::Display) -> TranscriptionError {
    TranscriptionError::EngineFailed(error.to_string())
}

#[async_trait]
impl TranscriptionProvider for ParakeetUnifiedProvider {
    async fn transcribe(&self, audio: Vec<f32>) -> Result<TranscriptResult, TranscriptionError> {
        let pool = Arc::clone(&self.pool);
        let transcript = tokio::task::spawn_blocking(move || Self::run(&pool, audio))
            .await
            .map_err(failed)?
            .map_err(failed)?;
        Ok(TranscriptResult {
            confidence: Some(confidence(&transcript).unwrap_or(DEFAULT_CONFIDENCE)),
            text: transcript.text,
            is_partial: false,
        })
    }

    async fn is_model_loaded(&self) -> bool {
        self.pool.model().is_some()
    }

    async fn get_current_model(&self) -> Option<String> {
        Some(self.model.to_string())
    }

    fn provider_name(&self) -> &'static str {
        "Parakeet Unified"
    }
}

#[cfg(test)]
mod tests {
    use deino_stt::TimedToken;

    use super::*;

    fn token(confidence: Option<f32>) -> TimedToken {
        TimedToken {
            text: " w".into(),
            start_ms: 0,
            end_ms: 80,
            confidence,
        }
    }

    #[test]
    fn confidence_is_the_mean_of_the_tokens_that_have_one() {
        let transcript = BatchTranscript {
            text: "w w w".into(),
            tokens: vec![token(Some(0.5)), token(None), token(Some(1.0))],
        };

        assert_eq!(confidence(&transcript), Some(0.75));
        assert_eq!(confidence(&BatchTranscript::default()), None);
    }
}
