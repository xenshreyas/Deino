//! What can go wrong in the meetings feature.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, InvalidTransition, MeetingId, MeetingViewId};

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
    /// The meeting's status does not allow the requested change.
    #[error(transparent)]
    InvalidTransition(#[from] InvalidTransition),
    /// Another writer changed the meeting's status first; reload and retry.
    #[error("meeting {0} changed while it was being updated")]
    StatusChanged(MeetingId),
    /// The internal/external filter needs the user's own address.
    #[error("add your email address in Settings → Profile to filter by internal or external")]
    AudienceUnknown,
    /// No saved view has the requested ID.
    #[error("view {0} was not found")]
    ViewNotFound(MeetingViewId),
    /// Another view already has the name.
    #[error("a view with this name already exists")]
    DuplicateViewName,
    /// The view name is blank or too long.
    #[error("a view name {0}")]
    InvalidViewName(&'static str),
    /// A merge names fewer than two different meetings.
    #[error("choose at least two meetings to merge")]
    MergeNeedsTwo,
    /// The meeting is still recording or processing, or is in the trash.
    #[error("meeting {0} cannot be merged until it has finished processing")]
    NotMergeable(MeetingId),
    /// The database failed; the source carries the detail.
    #[error("could not load or save meetings")]
    Storage(#[from] DbError),
}

impl ErrorClass for MeetingError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) | Self::ViewNotFound(_) => ErrorCode::NotFound,
            Self::EmptyTitle
            | Self::TitleTooLong
            | Self::AudienceUnknown
            | Self::InvalidViewName(_)
            | Self::MergeNeedsTwo => ErrorCode::Invalid,
            Self::InvalidTransition(_)
            | Self::StatusChanged(_)
            | Self::DuplicateViewName
            | Self::NotMergeable(_) => ErrorCode::Conflict,
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
