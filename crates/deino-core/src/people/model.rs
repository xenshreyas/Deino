//! People, their email addresses, and who attended which meeting.

use chrono::{DateTime, Utc};
use deino_domain::{AttendeeRole, CompanyId, Email, MeetingId, PersonId, Rsvp};
use serde::Serialize;

/// Someone the user meets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    /// The person's ID.
    pub id: PersonId,
    /// Their display name.
    pub name: String,
    /// Their email address; unique, so calendar attendees upsert onto one person.
    pub email: Option<Email>,
    /// Where they work.
    pub company_id: Option<CompanyId>,
    /// Their job title.
    pub title: Option<String>,
    /// A locally cached avatar.
    pub avatar_path: Option<String>,
    /// The user's own notes, in Markdown.
    pub notes_md: String,
    /// The AI summary of who they are and where things stand (ORG-04).
    pub summary_md: Option<String>,
    /// When `summary_md` was last refreshed.
    pub summary_updated_at: Option<DateTime<Utc>>,
    /// True for the user; at most one person is.
    pub is_me: bool,
    /// The person this one was merged into; merged people are hidden from lists.
    pub merged_into: Option<PersonId>,
    /// When the person was added.
    pub created_at: DateTime<Utc>,
    /// When the person last changed.
    pub updated_at: DateTime<Utc>,
}

/// A person's place in a meeting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Attendee {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The person.
    pub person_id: PersonId,
    /// How they took part.
    pub role: AttendeeRole,
    /// Their answer to the invitation; `None` when they were not invited or
    /// the calendar did not say.
    pub rsvp: Option<Rsvp>,
}
