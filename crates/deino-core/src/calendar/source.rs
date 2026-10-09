//! The [`CalendarSource`] port: where calendar data comes from.
//!
//! An adapter reads one provider (macOS Calendar through `EventKit`, the
//! Google Calendar API, Microsoft Graph) and reports what it sees in a
//! [`SyncWindow`] as plain values: accounts, their calendars, and each
//! calendar's events with their participants. It applies no rules: what to
//! hide, which link is the meeting link, who is external and which series an
//! event belongs to are decided in the core (`mapping.rs`, `filter.rs`), the
//! same way for every provider.

use std::error::Error;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{CalendarProvider, ErrorClass, ErrorCode, Rsvp};
use serde::Serialize;

use super::CalendarColor;

/// How far back a sync reads: meetings of the last month can still be
/// linked to their events (ORG-03).
pub const SYNC_PAST: TimeDelta = TimeDelta::days(30);
/// How far ahead a sync reads: "Coming up" shows the next two weeks.
pub const SYNC_AHEAD: TimeDelta = TimeDelta::days(14);

/// Whether Deino may read a source's calendars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CalendarAccess {
    /// The user has not been asked yet.
    NotDetermined,
    /// Full read access.
    Granted,
    /// The user said no; only System Settings can change it now.
    Denied,
    /// A device policy forbids access.
    Restricted,
    /// Only adding events is allowed, which is not enough to read them.
    WriteOnly,
    /// This platform has no such source (macOS Calendar off macOS).
    Unsupported,
}

/// The span a sync reads: `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncWindow {
    /// The earliest instant read.
    pub start: DateTime<Utc>,
    /// The instant reading stops (exclusive).
    pub end: DateTime<Utc>,
}

impl SyncWindow {
    /// The window around `now`: [`SYNC_PAST`] back and [`SYNC_AHEAD`] ahead.
    pub fn around(now: DateTime<Utc>) -> Self {
        Self {
            start: now - SYNC_PAST,
            end: now + SYNC_AHEAD,
        }
    }
}

/// Everything a source holds in one window.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceSnapshot {
    /// Every account, each with all its calendars (even empty ones).
    pub accounts: Vec<SourceAccount>,
}

/// One account of a source ("iCloud", "ada@acme.com", "On My Mac").
#[derive(Clone, Debug, PartialEq)]
pub struct SourceAccount {
    /// What the provider calls it; unique per provider, so syncs upsert.
    pub name: String,
    /// Its calendars.
    pub calendars: Vec<SourceCalendar>,
}

/// One calendar of an account.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceCalendar {
    /// The provider's stable ID for it.
    pub provider_id: String,
    /// Its name.
    pub title: String,
    /// Its colour.
    pub color: Option<CalendarColor>,
    /// Its events overlapping the window; a recurring event appears once per
    /// occurrence.
    pub events: Vec<SourceEvent>,
}

/// One event (or one occurrence of a recurring event) as the provider has it.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceEvent {
    /// The provider's ID for this occurrence; unique within its account.
    pub provider_event_id: String,
    /// The iCalendar UID shared by every copy of the invitation.
    pub ical_uid: Option<String>,
    /// The key every occurrence of its recurring series shares, or `None`
    /// for a one-off event.
    pub series_key: Option<String>,
    /// The title as written; may be empty.
    pub title: String,
    /// The notes or description, plain text or HTML.
    pub notes: Option<String>,
    /// The location field.
    pub location: Option<String>,
    /// The event's URL field.
    pub url: Option<String>,
    /// A video-call link from the provider's structured conference data.
    pub conference_url: Option<String>,
    /// When it starts.
    pub start: DateTime<Utc>,
    /// When it ends.
    pub end: DateTime<Utc>,
    /// True for an all-day event.
    pub all_day: bool,
    /// Whether it is on, tentative or cancelled.
    pub status: EventStatus,
    /// What kind of block it is, when the provider says (Google's event
    /// type); `None` lets the core infer it from the title and availability.
    pub kind: Option<EventKind>,
    /// How the event marks the user's time.
    pub availability: Availability,
    /// Who organised it.
    pub organizer: Option<Participant>,
    /// The invitation list (organiser included or not, as the provider has it).
    pub attendees: Vec<Participant>,
}

/// Whether an event takes place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventStatus {
    /// It takes place (or the provider does not say).
    Confirmed,
    /// It may take place.
    Tentative,
    /// It was called off.
    Cancelled,
}

/// What kind of calendar block an event is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// An ordinary event: a meeting, a call, a reminder to self.
    Default,
    /// Out of office.
    OutOfOffice,
    /// Focus time.
    Focus,
    /// Where the user works from that day.
    WorkingLocation,
}

/// How an event marks the user's time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    /// Busy.
    Busy,
    /// Free.
    Free,
    /// Tentatively busy.
    Tentative,
    /// Away: Exchange's "out of office".
    Unavailable,
    /// The provider does not say.
    Unknown,
}

/// Someone (or something) invited to an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Participant {
    /// Their email address as the provider wrote it.
    pub email: Option<String>,
    /// Their display name.
    pub name: Option<String>,
    /// Their answer to the invitation.
    pub rsvp: Option<Rsvp>,
    /// A person, or a room or resource booked for the event.
    pub kind: ParticipantKind,
    /// True for the user's own entry.
    pub is_self: bool,
}

/// What a participant is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticipantKind {
    /// A person (or the provider does not say).
    Person,
    /// A meeting room.
    Room,
    /// Equipment or another bookable resource.
    Resource,
    /// A mailing list.
    Group,
}

/// Why a source could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SourceError {
    /// The user has not granted access, or revoked it.
    #[error("Deino does not have access to your calendars")]
    AccessDenied,
    /// The source does not exist on this platform.
    #[error("this calendar source is not available on this computer")]
    Unsupported,
    /// Reading failed; the source carries the detail for the log.
    #[error("could not read your calendars")]
    Failed(#[source] Box<dyn Error + Send + Sync>),
}

impl ErrorClass for SourceError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::AccessDenied => ErrorCode::PermissionDenied,
            Self::Unsupported | Self::Failed(_) => ErrorCode::Unavailable,
        }
    }
}

/// A provider of calendars and events.
#[async_trait]
pub trait CalendarSource: Send + Sync {
    /// The provider this source reads.
    fn provider(&self) -> CalendarProvider;

    /// Whether Deino may read it now. Never prompts.
    async fn access(&self) -> CalendarAccess;

    /// Asks the user for access if they were never asked (the macOS prompt),
    /// and returns the answer; otherwise returns the current state.
    async fn request_access(&self) -> Result<CalendarAccess, SourceError>;

    /// Every account, calendar and event overlapping `window`.
    async fn fetch(&self, window: SyncWindow) -> Result<SourceSnapshot, SourceError>;

    /// Opens the system page where the user grants or revokes this source's
    /// access (System Settings → Privacy & Security → Calendars). False when
    /// the source has none or it could not be opened.
    fn open_access_settings(&self) -> bool {
        false
    }
}
