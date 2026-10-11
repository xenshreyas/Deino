//! What can go wrong starting, stopping or recovering a recording.

use deino_audio::{RecordError, RecoveryError, SourceError};
use deino_core::meetings::MeetingError;
use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};

use crate::engine::EngineError;

/// A recording failure. Messages are user-safe: no titles, no transcript.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RecordingError {
    /// A recording is already running (or starting, or stopping).
    #[error("a recording is already in progress")]
    AlreadyRecording,
    /// There is no recording to stop, pause or resume.
    #[error("nothing is being recorded")]
    NotRecording,
    /// The microphone could not start.
    #[error("the microphone could not start")]
    Capture(#[from] SourceError),
    /// A transcription engine could not start (a missing model, say).
    #[error(transparent)]
    Engine(#[from] EngineError),
    /// The recording files could not be created or closed.
    #[error("the recording files could not be written")]
    Audio(#[from] RecordError),
    /// A recording could not be repaired at launch.
    #[error("the recording could not be recovered")]
    Recovery(#[from] RecoveryError),
    /// The meeting could not be created or moved on.
    #[error(transparent)]
    Meeting(#[from] MeetingError),
    /// The database failed.
    #[error("could not save the recording")]
    Storage(#[from] DbError),
    /// A worker thread could not start or stopped unexpectedly.
    #[error("a recording thread failed")]
    Thread(#[from] std::io::Error),
}

impl ErrorClass for RecordingError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::AlreadyRecording | Self::NotRecording => ErrorCode::Conflict,
            Self::Capture(_) | Self::Engine(_) => ErrorCode::Unavailable,
            Self::Meeting(error) => error.code(),
            Self::Audio(_) | Self::Recovery(_) | Self::Storage(_) | Self::Thread(_) => {
                ErrorCode::Internal
            }
        }
    }
}
