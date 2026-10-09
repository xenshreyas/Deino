//! Calendar accounts and the events synced from them.

use chrono::{DateTime, Utc};
use deino_domain::{
    CalendarAccountId, CalendarEventId, CalendarProvider, ConferencePlatform, Rsvp, SeriesId,
};
use serde::{Deserialize, Serialize};

/// A calendar source the user connected.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarAccount {
    /// The account's ID.
    pub id: CalendarAccountId,
    /// Where its events come from.
    pub provider: CalendarProvider,
    /// What Settings shows: the account's email, or "On My Mac" for `EventKit`.
    /// Unique per provider.
    pub name: String,
    /// When the account was connected.
    pub created_at: DateTime<Utc>,
}

/// One event, as last synced from its calendar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    /// The event's ID in Deino.
    pub id: CalendarEventId,
    /// The account it was synced from.
    pub account_id: CalendarAccountId,
    /// The provider's ID for it; unique per account, so syncs upsert.
    pub provider_event_id: String,
    /// The iCalendar UID shared by every copy of the invitation.
    pub ical_uid: Option<String>,
    /// The recurring series it belongs to.
    pub series_id: Option<SeriesId>,
    /// The event's title; may be empty.
    pub title: String,
    /// The event's description; may be empty.
    pub description: String,
    /// When it starts.
    pub start_at: DateTime<Utc>,
    /// When it ends; never before `start_at`.
    pub end_at: DateTime<Utc>,
    /// The video-call link, if any.
    pub conference_url: Option<String>,
    /// The service behind `conference_url`.
    pub platform: Option<ConferencePlatform>,
    /// Who organised it.
    pub organizer_email: Option<String>,
    /// The invitation list as the provider sent it.
    pub attendees: Vec<EventAttendee>,
    /// The user's own answer.
    pub my_rsvp: Option<Rsvp>,
    /// True when someone outside the user's domains is invited.
    pub is_external: bool,
    /// `None` follows the auto-record rules; `Some(true)` always records,
    /// `Some(false)` never does. Set by the user, so syncs keep it.
    pub record_override: Option<bool>,
    /// When the event was first synced.
    pub created_at: DateTime<Utc>,
    /// When a sync last changed it.
    pub updated_at: DateTime<Utc>,
}

/// One invitee of a calendar event. Stored as JSON in
/// `calendar_events.attendees_json`; linking the event to a meeting turns
/// these into people and `meeting_attendees` rows (ORG-03, ORG-04).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EventAttendee {
    /// Their email address, as the provider wrote it.
    pub email: Option<String>,
    /// Their display name.
    pub name: Option<String>,
    /// Their answer to the invitation.
    pub rsvp: Option<Rsvp>,
    /// True for the organiser.
    pub is_organizer: bool,
}
