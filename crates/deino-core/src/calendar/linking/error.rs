//! What can go wrong linking a meeting to an event.
//!
//! Every variant's text is the user-safe message the UI receives: IDs only,
//! never a title or an attendee.

use deino_db::DbError;
use deino_domain::{CalendarEventId, ErrorClass, ErrorCode, MeetingId};

use crate::calendar::CalendarError;

/// A linking failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LinkError {
    /// No meeting has this ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// No stored event has this ID.
    #[error("calendar event {0} was not found")]
    EventNotFound(CalendarEventId),
    /// The calendar could not list its events.
    #[error(transparent)]
    Calendar(#[from] CalendarError),
    /// The database failed; the source carries the detail for the log.
    #[error("could not load or save the meeting's calendar link")]
    Storage(#[from] DbError),
}

impl ErrorClass for LinkError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::MeetingNotFound(_) | Self::EventNotFound(_) => ErrorCode::NotFound,
            Self::Calendar(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;

    #[test]
    fn missing_meetings_and_events_are_not_found_and_storage_is_internal() {
        let clock = FakeClock::new(at("2026-10-09T10:00:00Z"));
        let meeting = LinkError::MeetingNotFound(MeetingId::new("meeting-1".to_owned()));
        let event = LinkError::EventNotFound(CalendarEventId::generate(&clock));

        assert_eq!(meeting.code(), ErrorCode::NotFound);
        assert_eq!(event.code(), ErrorCode::NotFound);
        assert_eq!(meeting.to_string(), "meeting meeting-1 was not found");
        assert_eq!(
            LinkError::Calendar(CalendarError::InvalidRange).code(),
            ErrorCode::Invalid
        );
    }
}
