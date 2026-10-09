//! What can go wrong in the tags feature.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId, TagId};
use deino_llm::LlmError;

use super::{MAX_TAG_DESCRIPTION_CHARS, MAX_TAG_NAME_CHARS};

/// A tags failure. Messages are user-safe: IDs and limits, never tag names
/// or meeting content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TagError {
    /// No tag has the requested ID.
    #[error("tag {0} was not found")]
    NotFound(TagId),
    /// No meeting has the requested ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// The tag name is blank once whitespace is trimmed.
    #[error("a tag name cannot be empty")]
    EmptyName,
    /// The tag name is longer than [`MAX_TAG_NAME_CHARS`].
    #[error("a tag name can be at most {MAX_TAG_NAME_CHARS} characters long")]
    NameTooLong,
    /// The description is longer than [`MAX_TAG_DESCRIPTION_CHARS`].
    #[error("a tag description can be at most {MAX_TAG_DESCRIPTION_CHARS} characters long")]
    DescriptionTooLong,
    /// Another tag already has the name, ignoring case.
    #[error("a tag with that name already exists")]
    NameTaken,
    /// The auto-tagger's model call failed; the source says how.
    #[error(transparent)]
    Llm(#[from] LlmError),
    /// The database failed; the source carries the detail.
    #[error("could not load or save tags")]
    Storage(#[from] DbError),
}

impl ErrorClass for TagError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) | Self::MeetingNotFound(_) => ErrorCode::NotFound,
            Self::EmptyName | Self::NameTooLong | Self::DescriptionTooLong => ErrorCode::Invalid,
            Self::NameTaken => ErrorCode::Conflict,
            Self::Llm(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}

impl TagError {
    /// `error` as [`TagError::NameTaken`] when it is a name clash, else as a
    /// storage failure.
    pub(super) fn naming(error: DbError) -> Self {
        if error.is_unique_violation() {
            Self::NameTaken
        } else {
            Self::Storage(error)
        }
    }

    /// True when trying again later may succeed: a busy database, or a
    /// provider that is rate limited, overloaded or unreachable.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Llm(error) => error.is_retryable(),
            Self::Storage(_) => true,
            _ => false,
        }
    }
}
