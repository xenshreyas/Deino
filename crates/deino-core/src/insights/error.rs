//! What can go wrong with insights. Messages name fields and IDs, never
//! what the user wrote or what a meeting said.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, InsightId, InvalidCondition, MeetingId};
use deino_llm::LlmError;

/// An insights failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InsightError {
    /// No insight has the ID.
    #[error("insight {0} was not found")]
    NotFound(InsightId),
    /// No meeting outside the trash has the ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// No gallery template has the key.
    #[error("there is no insight template {0:?}")]
    TemplateNotFound(String),
    /// A field of the definition breaks a rule.
    #[error("{field} {rule}")]
    Invalid {
        /// The field, for example "name".
        field: &'static str,
        /// What is wrong, for example "cannot be empty".
        rule: &'static str,
    },
    /// A condition cannot match as written.
    #[error(transparent)]
    Condition(#[from] InvalidCondition),
    /// Another insight already has the name.
    #[error("an insight with this name already exists")]
    DuplicateName,
    /// The model could not answer; the gateway's message says why.
    #[error(transparent)]
    Llm(#[from] LlmError),
    /// The post-meeting pipeline would not take the run.
    #[error("could not schedule the run: {0}")]
    Schedule(String),
    /// The CSV could not be written.
    #[error("could not save the file")]
    Write(#[source] std::io::Error),
    /// The database failed; the source carries the detail.
    #[error("could not load or save insights")]
    Storage(#[from] DbError),
}

impl InsightError {
    /// A broken rule of `field`.
    pub fn invalid(field: &'static str, rule: &'static str) -> Self {
        Self::Invalid { field, rule }
    }

    /// True for failures that may pass on their own (rate limits, a busy
    /// provider or database); a missing key or model needs the user.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Llm(error) => error.is_retryable(),
            Self::Storage(_) => true,
            _ => false,
        }
    }
}

impl ErrorClass for InsightError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) | Self::MeetingNotFound(_) | Self::TemplateNotFound(_) => {
                ErrorCode::NotFound
            }
            Self::Invalid { .. } | Self::Condition(_) => ErrorCode::Invalid,
            Self::DuplicateName => ErrorCode::Conflict,
            Self::Llm(error) => error.code(),
            Self::Schedule(_) => ErrorCode::Unavailable,
            Self::Write(_) => ErrorCode::Internal,
            Self::Storage(error) => error.code(),
        }
    }
}
