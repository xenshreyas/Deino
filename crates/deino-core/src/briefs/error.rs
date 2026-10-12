//! What can go wrong with pre-meeting briefs.

use deino_db::DbError;
use deino_domain::{CalendarEventId, ErrorClass, ErrorCode};
use deino_llm::LlmError;

use crate::calendar::CalendarError;
use crate::calendar::linking::LinkError;

/// A briefs failure. Messages are user-safe: IDs only, never names, titles
/// or meeting content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BriefError {
    /// No calendar event has the ID.
    #[error("calendar event {0} was not found")]
    EventNotFound(CalendarEventId),
    /// The calendar could not be read.
    #[error(transparent)]
    Calendar(#[from] CalendarError),
    /// The related meetings could not be read.
    #[error(transparent)]
    Related(#[from] LinkError),
    /// The model call failed; the source says how.
    #[error(transparent)]
    Llm(#[from] LlmError),
    /// The database failed; the source carries the detail.
    #[error("could not load or save the brief")]
    Storage(#[from] DbError),
}

impl ErrorClass for BriefError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::EventNotFound(_) => ErrorCode::NotFound,
            Self::Calendar(error) => error.code(),
            Self::Related(error) => error.code(),
            Self::Llm(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
