//! What can go wrong when the app asks the pipeline for something.
//!
//! Stage failures are not errors of this kind: they are stored on the job
//! and shown in the meeting's status strip.

use deino_core::meetings::MeetingError;
use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId, MeetingStatus, StageId};

/// A pipeline request failed. Messages are user-safe: IDs and statuses only.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PipelineError {
    /// No meeting has the requested ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// The requested stage is not registered.
    #[error("the pipeline has no stage named {0}")]
    UnknownStage(StageId),
    /// The meeting is not recorded yet, so there is nothing to process.
    #[error("meeting {meeting} cannot be processed while it is {status}")]
    NotRecorded {
        /// The meeting.
        meeting: MeetingId,
        /// Its status, `scheduled` or `recording`.
        status: MeetingStatus,
    },
    /// Moving the meeting back to `processing` failed.
    #[error(transparent)]
    Meeting(#[from] MeetingError),
    /// The database failed; the source carries the detail.
    #[error("could not load or save pipeline jobs")]
    Storage(#[from] DbError),
}

impl ErrorClass for PipelineError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::MeetingNotFound(_) => ErrorCode::NotFound,
            Self::UnknownStage(_) => ErrorCode::Invalid,
            Self::NotRecorded { .. } => ErrorCode::Conflict,
            Self::Meeting(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
