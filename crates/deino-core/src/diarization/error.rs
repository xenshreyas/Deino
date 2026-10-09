//! What can go wrong in the diarize stage.

use deino_db::DbError;
use deino_domain::{Channel, ErrorClass, ErrorCode, MeetingId};

use super::TrackError;

/// A diarization failure. Messages carry IDs and channels, never content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DiarizationError {
    /// No meeting has the requested ID.
    #[error("meeting {0} was not found")]
    NotFound(MeetingId),
    /// The planned track has no finalized audio file.
    #[error("the {0} recording of the meeting is missing")]
    AudioMissing(Channel),
    /// The engine failed.
    #[error(transparent)]
    Track(#[from] TrackError),
    /// The database failed.
    #[error("could not load or save the transcript's speakers")]
    Storage(#[from] DbError),
}

impl ErrorClass for DiarizationError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) | Self::AudioMissing(_) => ErrorCode::NotFound,
            Self::Track(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
