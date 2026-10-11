//! What can go wrong with action items. Messages name fields and IDs, never
//! what the user wrote.

use deino_db::DbError;
use deino_domain::{ActionItemId, ActionItemViewId, ErrorClass, ErrorCode, MeetingId, PersonId};

/// An action items failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ActionItemError {
    /// No item has the ID.
    #[error("action item {0} was not found")]
    NotFound(ActionItemId),
    /// No saved view has the ID.
    #[error("view {0} was not found")]
    ViewNotFound(ActionItemViewId),
    /// No meeting has the ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// No person has the ID.
    #[error("person {0} was not found")]
    PersonNotFound(PersonId),
    /// A field breaks a rule.
    #[error("{field} {rule}")]
    Invalid {
        /// The field, for example "title".
        field: &'static str,
        /// What is wrong, for example "cannot be empty".
        rule: &'static str,
    },
    /// Another saved view already has the name.
    #[error("a view with this name already exists")]
    DuplicateViewName,
    /// The database failed; the source carries the detail.
    #[error("could not load or save action items")]
    Storage(#[from] DbError),
}

impl ActionItemError {
    /// A broken rule of `field`.
    pub fn invalid(field: &'static str, rule: &'static str) -> Self {
        Self::Invalid { field, rule }
    }
}

impl ErrorClass for ActionItemError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_)
            | Self::ViewNotFound(_)
            | Self::MeetingNotFound(_)
            | Self::PersonNotFound(_) => ErrorCode::NotFound,
            Self::Invalid { .. } => ErrorCode::Invalid,
            Self::DuplicateViewName => ErrorCode::Conflict,
            Self::Storage(error) => error.code(),
        }
    }
}
