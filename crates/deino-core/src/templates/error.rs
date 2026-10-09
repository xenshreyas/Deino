//! What can go wrong with templates. Messages name fields, positions and
//! IDs, never what the user wrote.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId, TemplateId};

/// A templates failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TemplateError {
    /// No template has the ID.
    #[error("template {0} was not found")]
    NotFound(TemplateId),
    /// No meeting has the ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// Built-in templates cannot be edited or deleted.
    #[error("built-in templates cannot be changed; duplicate it to edit a copy")]
    BuiltinIsReadOnly(TemplateId),
    /// A field breaks a rule.
    #[error("{field} {rule}")]
    Invalid {
        /// The field, for example "name".
        field: &'static str,
        /// What is wrong, for example "cannot be empty".
        rule: &'static str,
    },
    /// A section breaks a rule.
    #[error("section {position} {rule}")]
    InvalidSection {
        /// The section's 1-based position.
        position: usize,
        /// What is wrong.
        rule: &'static str,
    },
    /// An auto rule breaks a rule.
    #[error("auto rule {position} {rule}")]
    InvalidRule {
        /// The rule's 1-based position.
        position: usize,
        /// What is wrong.
        rule: &'static str,
    },
    /// An imported file is neither a Deino template nor a legacy one.
    #[error("this file is not a Deino template")]
    UnreadableDocument(#[source] serde_json::Error),
    /// There is no template at all to choose from (only if the built-ins were
    /// never seeded).
    #[error("no templates are available yet")]
    NoTemplates,
    /// A legacy templates folder could not be read.
    #[error("could not read the old templates folder")]
    LegacyFolder(#[source] std::io::Error),
    /// The database failed; the source carries the detail.
    #[error("could not load or save templates")]
    Storage(#[from] DbError),
}

impl TemplateError {
    /// A broken rule of `field`.
    pub(crate) fn invalid(field: &'static str, rule: &'static str) -> Self {
        Self::Invalid { field, rule }
    }
}

impl ErrorClass for TemplateError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) | Self::MeetingNotFound(_) => ErrorCode::NotFound,
            Self::BuiltinIsReadOnly(_) => ErrorCode::Conflict,
            Self::Invalid { .. }
            | Self::InvalidSection { .. }
            | Self::InvalidRule { .. }
            | Self::UnreadableDocument(_) => ErrorCode::Invalid,
            Self::NoTemplates => ErrorCode::Unavailable,
            Self::LegacyFolder(_) | Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
