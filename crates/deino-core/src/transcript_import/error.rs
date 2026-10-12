//! What can go wrong importing a transcript.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};

use crate::meetings::MeetingError;
use crate::tags::TagError;

/// An import failure. Messages are user-safe: never the transcript.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TranscriptImportError {
    /// The request breaks a rule; the text says which.
    #[error("the meeting cannot be imported: {0}")]
    Invalid(&'static str),
    /// The title is not a valid meeting title.
    #[error(transparent)]
    Meeting(#[from] MeetingError),
    /// A tag could not be found, made or applied.
    #[error(transparent)]
    Tag(#[from] TagError),
    /// The database failed; the source carries the detail.
    #[error("could not save the imported meeting")]
    Storage(#[from] DbError),
}

impl ErrorClass for TranscriptImportError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Invalid(_) => ErrorCode::Invalid,
            Self::Meeting(error) => error.code(),
            Self::Tag(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
