//! What one row of the Meetings page shows, and a page of rows.

use chrono::{DateTime, Utc};
use deino_domain::{MeetingId, MeetingStatus, PaletteColor, PersonId, Seconds, TagId};
use serde::Serialize;

use super::MeetingCursor;

/// The most rows one page may ask for.
pub const MAX_PAGE_SIZE: u32 = 200;

/// A meeting as its list row shows it: title, time, length, who was there
/// and its tags (Circleback §14: invitee avatars and colour-coded tags).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ListedMeeting {
    /// The meeting's ID.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it was saved; the list's order and the day it is grouped under.
    pub created_at: DateTime<Utc>,
    /// When the recording started, if known.
    pub started_at: Option<DateTime<Utc>>,
    /// The recorded length, if known.
    pub duration: Option<Seconds>,
    /// Where it is in its lifecycle.
    pub status: MeetingStatus,
    /// Who attended, the user last.
    pub attendees: Vec<ListedAttendee>,
    /// Its tags, by name.
    pub tags: Vec<ListedTag>,
}

/// An attendee's avatar on a list row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ListedAttendee {
    /// The person.
    pub person_id: PersonId,
    /// Their name, for initials and the tooltip.
    pub name: String,
    /// Their cached avatar, if any.
    pub avatar_path: Option<String>,
    /// True for the user.
    pub is_me: bool,
}

/// A tag chip on a list row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ListedTag {
    /// The tag.
    pub id: TagId,
    /// Its name.
    pub name: String,
    /// Its palette colour.
    pub color: PaletteColor,
}

/// One page of the list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPage {
    /// The rows, newest first.
    pub meetings: Vec<ListedMeeting>,
    /// Where the next page starts; `None` on the last page.
    pub next: Option<MeetingCursor>,
}

/// A row of the page query, before its attendees and tags are attached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingRow {
    /// The meeting's ID.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// `created_at` as stored, for the cursor.
    pub cursor_at: String,
    /// `created_at` parsed.
    pub created_at: DateTime<Utc>,
    /// When the recording started.
    pub started_at: Option<DateTime<Utc>>,
    /// The recorded length.
    pub duration: Option<Seconds>,
    /// Its status.
    pub status: MeetingStatus,
}

impl MeetingRow {
    /// The cursor that starts the page after this row.
    pub fn cursor(&self) -> MeetingCursor {
        MeetingCursor {
            at: self.cursor_at.clone(),
            id: self.id.clone(),
        }
    }

    /// The list row, with what was read about it.
    pub fn listed(self, attendees: Vec<ListedAttendee>, tags: Vec<ListedTag>) -> ListedMeeting {
        ListedMeeting {
            id: self.id,
            title: self.title,
            created_at: self.created_at,
            started_at: self.started_at,
            duration: self.duration,
            status: self.status,
            attendees,
            tags,
        }
    }
}

/// An attendee read for a page, with the meeting it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttendeeRow {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The avatar.
    pub attendee: ListedAttendee,
}

/// A tag read for a page, with the meeting it is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagRow {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The chip.
    pub tag: ListedTag,
}
