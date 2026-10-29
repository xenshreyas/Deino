//! The track transcription port: one finished track in, timed speech out.
//! `deino-recording`'s `OfflineTranscriber` implements it over the
//! `deino-stt` batch engine; tests script it.

use std::path::PathBuf;

use async_trait::async_trait;
use deino_domain::{Channel, ErrorClass, ErrorCode, Millis};
use tokio_util::sync::CancellationToken;

use super::TrackProgress;
use crate::segments::WordTiming;

/// A boxed engine error (the engine's own types stay in its crate).
pub type EngineError = Box<dyn std::error::Error + Send + Sync>;

/// One finished track to transcribe.
#[derive(Clone, Debug)]
pub struct TrackRequest {
    /// The finalized file (`mic.m4a`, `system.m4a` or `audio.mp4`).
    pub audio: PathBuf,
    /// Which track it is: the transcriber checks the microphone's lines
    /// for a voice (`deino-recording::offline::silent_lines`).
    pub channel: Channel,
    /// The system track, on the same timeline, when this is the microphone
    /// track of a meeting that has one: what the speakers played, whose
    /// echo the microphone may hold. The transcriber may cancel that echo
    /// from its copy of the audio first; the file is never changed.
    pub echo_reference: Option<PathBuf>,
    /// Fires when the stage is cancelled; checked between utterances.
    pub cancel: CancellationToken,
    /// How far through the track the transcriber is: the audio position
    /// done over the track's length, ending at 1.0.
    pub progress: TrackProgress,
}

/// One utterance heard on a track, on the recording's timeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeardSpan {
    /// When it starts.
    pub start: Millis,
    /// When it ends; never before `start`.
    pub end: Millis,
    /// What was said.
    pub text: String,
    /// Its words, when the engine times them.
    pub words: Vec<WordTiming>,
}

/// Why a track could not be transcribed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TrackError {
    /// No transcription model is downloaded, or the engine cannot start.
    #[error("the transcription model is not available: download it in Settings → Transcription")]
    Unavailable(#[source] EngineError),
    /// Decoding the track or running the engine failed.
    #[error("transcribing the recording failed")]
    Failed(#[source] EngineError),
    /// The run was cancelled.
    #[error("transcription was cancelled")]
    Cancelled,
}

impl ErrorClass for TrackError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Unavailable(_) => ErrorCode::Unavailable,
            Self::Failed(_) => ErrorCode::Internal,
            Self::Cancelled => ErrorCode::Cancelled,
        }
    }
}

/// Transcribes finished tracks.
#[async_trait]
pub trait TrackTranscriber: Send + Sync {
    /// The utterances of one track, in time order. Heavy: implementations
    /// run it off the async runtime's worker threads.
    async fn transcribe(&self, request: TrackRequest) -> Result<Vec<HeardSpan>, TrackError>;
}
