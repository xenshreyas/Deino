//! What can go wrong in calendar sync and its commands.
//!
//! Every variant's text is the user-safe message the UI receives: IDs and
//! limits, never an event's title, notes or attendees.

use deino_db::DbError;
use deino_domain::{CalendarEventId, CalendarId, CalendarProvider, ErrorClass, ErrorCode};

use super::SourceError;
use crate::settings::SettingsError;

/// The longest span `upcoming` lists, in days.
pub const MAX_RANGE_DAYS: i64 = 62;

/// A calendar failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CalendarError {
    /// No stored event has this ID.
    #[error("calendar event {0} was not found")]
    EventNotFound(CalendarEventId),
    /// No stored calendar has this ID.
    #[error("calendar {0} was not found")]
    CalendarNotFound(CalendarId),
    /// No direct connector serves this provider.
    #[error("there is no {0} calendar connector")]
    NoConnector(CalendarProvider),
    /// The range ends before it starts, or spans too long.
    #[error("a time range must end after it starts and span at most {MAX_RANGE_DAYS} days")]
    InvalidRange,
    /// The calendar source failed or refused access.
    #[error(transparent)]
    Source(#[from] SourceError),
    /// The sync setting could not be saved.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The database failed; the source carries the detail for the log.
    #[error("could not load or save calendars")]
    Storage(#[from] DbError),
}

impl ErrorClass for CalendarError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::EventNotFound(_) | Self::CalendarNotFound(_) => ErrorCode::NotFound,
            Self::InvalidRange | Self::NoConnector(_) => ErrorCode::Invalid,
            Self::Source(error) => error.code(),
            Self::Settings(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
