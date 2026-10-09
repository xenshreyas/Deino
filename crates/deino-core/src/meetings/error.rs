//! What can go wrong in the meetings feature.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};

use super::MAX_TITLE_CHARS;

/// A meetings failure. Messages are user-safe: IDs, never titles or content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MeetingError {
    /// No meeting has the requested ID.
    #[error("meeting {0} was not found")]
    NotFound(MeetingId),
    /// The new title is blank once whitespace is trimmed.
    #[error("a meeting title cannot be empty")]
    EmptyTitle,
    /// The new title is longer than [`MAX_TITLE_CHARS`].
    #[error("a meeting title can be at most {MAX_TITLE_CHARS} characters long")]
    TitleTooLong,
    /// The database failed; the source carries the detail.
    #[error("could not load or save meetings")]
    Storage(#[from] DbError),
}

impl ErrorClass for MeetingError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::EmptyTitle | Self::TitleTooLong => ErrorCode::Invalid,
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
