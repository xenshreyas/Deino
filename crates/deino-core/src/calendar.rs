//! Calendar accounts and synced events.
//!
//! [`CalendarAccountRepo`] stores the calendar sources the user connected;
//! [`CalendarEventRepo`] stores their events, upserting by the provider's
//! ID so every sync refreshes rather than duplicates. [`SqliteCalendarRepo`]
//! implements both and `FakeCalendarRepo` stands in for tests.
//!
//! A meeting points at its event through `meetings.calendar_event_id`
//! ([`MeetingLifecycleRepo::link`](crate::meetings::MeetingLifecycleRepo::link));
//! events do not point back (ADR 0012). ORG-01 fills these tables from
//! `EventKit`, Google and Microsoft; ORG-02 and ORG-03 read them.

mod account_repo;
#[cfg(test)]
mod calendar_tests;
mod event_repo;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;

pub use account_repo::{CalendarAccountRepo, SqliteCalendarRepo};
pub use event_repo::CalendarEventRepo;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeCalendarRepo;
pub use model::{CalendarAccount, CalendarEvent, EventAttendee};
