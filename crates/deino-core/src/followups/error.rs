//! What can go wrong with suggested follow-ups.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};
use deino_llm::LlmError;

use crate::export::ExportError;

/// A follow-up failure. Messages are user-safe: IDs only, never names,
/// addresses or meeting content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FollowUpError {
    /// The meeting has no ready draft (none was written, or it was dismissed).
    #[error("meeting {0} has no follow-up draft")]
    NoDraft(MeetingId),
    /// A recipient the user typed is not an email address.
    #[error("a recipient is not a valid email address")]
    InvalidRecipient,
    /// The draft has nobody to send it to.
    #[error("add at least one recipient")]
    NoRecipients,
    /// The draft is too long for a `mailto:` link; copying still works.
    #[error("the email is too long to open in Mail; copy it instead")]
    TooLongForMail,
    /// Mail could not be opened, or the clipboard written.
    #[error(transparent)]
    Platform(#[from] ExportError),
    /// The model call failed; the source says how.
    #[error(transparent)]
    Llm(#[from] LlmError),
    /// The database failed; the source carries the detail.
    #[error("could not load or save the follow-up")]
    Storage(#[from] DbError),
}

impl FollowUpError {
    /// True when trying again later may succeed: a busy or rate-limited
    /// provider, or a busy database.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Llm(error) => error.is_retryable(),
            Self::Storage(_) => true,
            _ => false,
        }
    }
}

impl ErrorClass for FollowUpError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NoDraft(_) => ErrorCode::NotFound,
            Self::InvalidRecipient | Self::NoRecipients | Self::TooLongForMail => {
                ErrorCode::Invalid
            }
            Self::Platform(error) => error.code(),
            Self::Llm(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
