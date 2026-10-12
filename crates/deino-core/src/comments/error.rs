//! What can go wrong with a comment.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};

/// A comment failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommentError {
    /// The database failed; the source carries the detail.
    #[error("could not save the comment")]
    Storage(#[from] DbError),
    /// The comment is empty.
    #[error("write a comment first")]
    EmptyBody,
    /// The span is empty, or its target and line do not fit together (a
    /// transcript thread without a line of this meeting, a notes thread
    /// with one).
    #[error("select some text to comment on")]
    InvalidAnchor,
    /// No such thread or comment (or its meeting is in the trash).
    #[error("this comment no longer exists")]
    NotFound,
}

impl ErrorClass for CommentError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Storage(_) => ErrorCode::Internal,
            Self::EmptyBody | Self::InvalidAnchor => ErrorCode::Invalid,
            Self::NotFound => ErrorCode::NotFound,
        }
    }
}
