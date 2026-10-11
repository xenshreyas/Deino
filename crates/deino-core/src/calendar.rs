//! Calendar sync (ORG-01): accounts, calendars and events from the user's
//! calendars, and the rules for reading them.
//!
//! - **Sources.** [`CalendarSource`] is the port a provider adapter
//!   implements: macOS Calendar through `EventKit` (the default, every account
//!   the Mac knows, no OAuth) in `deino-eventkit`; Google and Microsoft
//!   connectors later. Adapters report plain values and apply no rules.
//! - **Rules.** [`mapping`](self::mapping) reads every provider's events the
//!   same way: the join link ([`find_conference`]), people but no rooms,
//!   the user's RSVP, internal vs external (NOT-05's
//!   [`InternalDomains`](crate::profile::InternalDomains)) and the
//!   recurring series. [`filter`](self::filter) drops cancelled events and
//!   out-of-office, focus and working-location blocks at sync, and hides
//!   declined events, hidden calendars and duplicates on read.
//! - **Storage.** [`CalendarSyncRepo`] stores a [`SyncBatch`] in one
//!   transaction, touching only what changed; [`CalendarAccountRepo`],
//!   [`CalendarRepo`] and [`CalendarEventRepo`] read it back.
//!   [`SqliteCalendarRepo`] implements all four and `FakeCalendarRepo`
//!   stands in for tests. [`AttendeeDirectory`] adds invitees to people and
//!   companies.
//! - **Service.** [`CalendarService`] (the `calendar` plugin's commands) and
//!   [`SyncScheduler`] (every two minutes, on change, on wake).
//!
//! A meeting points at its event through `meetings.calendar_event_id`
//! ([`MeetingLifecycleRepo::link`](crate::meetings::MeetingLifecycleRepo::link));
//! events do not point back (ADR 0012). ORG-02 and ORG-03 read these tables.

mod account_repo;
mod batch;
mod calendar_repo;
#[cfg(test)]
mod calendar_tests;
mod color;
mod conference;
mod connectors;
mod entities;
mod error;
mod event_repo;
mod events;
#[cfg(any(test, feature = "test-support"))]
mod fake;
#[cfg(any(test, feature = "test-support"))]
mod fake_source;
#[cfg(any(test, feature = "test-support"))]
mod fake_sync;
mod filter;
mod mapping;
mod model;
mod names;
mod reads;
mod schedule;
mod service;
#[cfg(test)]
mod service_tests;
mod source;
mod sync;
mod sync_repo;
#[cfg(test)]
mod sync_repo_tests;
#[cfg(test)]
mod sync_tests;
#[cfg(test)]
mod test_events;
mod view;

pub use account_repo::{CalendarAccountRepo, SqliteCalendarRepo};
pub use batch::{SyncBatch, SyncReport, SyncedAccount, SyncedCalendar, SyncedEvent};
pub use calendar_repo::CalendarRepo;
pub use color::{CalendarColor, InvalidCalendarColor};
pub use conference::{Conference, ConferenceFields, MeetingService, find_conference};
pub use entities::{AttendeeDirectory, MAX_ATTENDEES_FOR_PEOPLE};
pub use error::{CalendarError, MAX_RANGE_DAYS};
pub use event_repo::CalendarEventRepo;
pub use events::CalendarEvents;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeCalendarRepo;
#[cfg(any(test, feature = "test-support"))]
pub use fake_source::{FakeCalendarSource, RecordedCalendarEvents};
pub use filter::TimeRange;
pub use model::{Calendar, CalendarAccount, CalendarEvent, EventAttendee};
pub use schedule::{REFRESH_INTERVAL, SETTLE_DELAY, SyncScheduler, SyncTrigger, SyncTriggers};
pub use service::{CalendarDeps, CalendarService, CalendarStore};
pub use source::{
    Availability, CalendarAccess, CalendarSource, EventKind, EventStatus, Participant,
    ParticipantKind, SYNC_AHEAD, SYNC_PAST, SourceAccount, SourceCalendar, SourceError,
    SourceEvent, SourceSnapshot, SyncWindow,
};
pub use sync::CalendarSync;
pub use sync_repo::CalendarSyncRepo;
pub use view::{CalendarStatus, CalendarView, ConnectorStatus};
