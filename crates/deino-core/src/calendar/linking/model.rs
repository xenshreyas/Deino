//! What linking reports, and how related meetings are described.

use chrono::{DateTime, Utc};
use deino_domain::MeetingId;
use serde::Serialize;

use crate::calendar::CalendarEvent;

/// Whether linking may rename the meeting after its event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleRule {
    /// Take the event's title (the picker; a recording started untitled).
    FromEvent,
    /// Keep the title the meeting was started with.
    Keep,
}

/// What a link, re-link or unlink changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EventLinkChange {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The event the meeting is linked to now; `None` after an unlink.
    pub event: Option<CalendarEvent>,
    /// The meeting's new title, when linking renamed it after the event.
    pub title: Option<String>,
    /// People the event added to the meeting's attendees.
    pub attendees_added: u32,
    /// The previous event's invitees taken off the meeting.
    pub attendees_removed: u32,
}

/// Why two meetings are related.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RelatedBy {
    /// Both were recorded for the same recurring calendar series.
    Series,
    /// Both have exactly the same title and the same people.
    TitleAndPeople,
    /// Both have exactly the same people, whatever the titles ("Review
    /// last meeting" on an upcoming event, ORG-02).
    People,
}

/// A meeting related to the one open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RelatedMeeting {
    /// The related meeting.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it took place: when recording started, or when it was created.
    pub happened_at: DateTime<Utc>,
    /// Why it is related.
    pub by: RelatedBy,
}

/// The meetings related to one meeting.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RelatedMeetings {
    /// The latest related meeting that took place before this one
    /// ("Previous in series").
    pub previous: Option<RelatedMeeting>,
    /// The other related meetings, newest first, at most
    /// [`RELATED_LIMIT`](super::RELATED_LIMIT).
    pub others: Vec<RelatedMeeting>,
}
