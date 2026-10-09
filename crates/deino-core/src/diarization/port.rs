//! The diarization engine port. `deino-diarize` implements it (decode the
//! track, run the models, prune phantoms); tests use
//! [`FakeTrackDiarizer`](super::FakeTrackDiarizer).

use async_trait::async_trait;
use deino_domain::{ErrorClass, ErrorCode};

use super::{TrackDiarization, TrackRequest};

/// A boxed engine error (the engine's own types stay in its crate).
pub type EngineError = Box<dyn std::error::Error + Send + Sync>;

/// Why a track could not be diarized.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TrackError {
    /// The speaker models are not downloaded or fail verification.
    #[error("the speaker models are not available")]
    ModelsUnavailable(#[source] EngineError),
    /// Decoding or running the models failed.
    #[error("speaker diarization failed")]
    Failed(#[source] EngineError),
    /// The run was cancelled.
    #[error("speaker diarization was cancelled")]
    Cancelled,
}

impl ErrorClass for TrackError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::ModelsUnavailable(_) => ErrorCode::Unavailable,
            Self::Failed(_) => ErrorCode::Internal,
            Self::Cancelled => ErrorCode::Cancelled,
        }
    }
}

/// Finds who spoke when on one recorded track.
#[async_trait]
pub trait TrackDiarizer: Send + Sync {
    /// The embedding model the voices' centroids come from, stored beside
    /// them so vectors of different models are never compared.
    fn embedding_model(&self) -> &str;

    /// Diarizes the track. CPU-heavy: implementations run it off the async
    /// runtime's worker threads.
    async fn diarize(&self, request: TrackRequest) -> Result<TrackDiarization, TrackError>;
}
