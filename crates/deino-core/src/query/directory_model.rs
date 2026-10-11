//! What the directory lists: people, companies, tags, folders and calendar
//! events, shaped like the meeting views ([`super::model`]).

use chrono::{DateTime, FixedOffset, NaiveDate};
use deino_domain::{CalendarEventId, CompanyId, FolderId, PersonId, TagId};
use schemars::JsonSchema;
use serde::Serialize;

/// A person the user has met or added.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersonView {
    /// Their ID; also accepted wherever a person is named.
    #[schemars(with = "String")]
    pub id: PersonId,
    /// Their name.
    pub name: String,
    /// Their email, when known.
    pub email: Option<String>,
    /// Their job title, when known.
    pub title: Option<String>,
    /// Their company's name, when known.
    pub company: Option<String>,
    /// True for the user.
    pub is_me: bool,
}

/// A company.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompanyView {
    /// Its ID; also accepted wherever a company is named.
    #[schemars(with = "String")]
    pub id: CompanyId,
    /// Its name.
    pub name: String,
    /// Its email domain, when known.
    pub domain: Option<String>,
    /// How many known people work there.
    pub people: u32,
}

/// A tag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagView {
    /// Its ID; also accepted wherever a tag is named.
    #[schemars(with = "String")]
    pub id: TagId,
    /// Its name.
    pub name: String,
    /// What it is for; may be empty.
    pub description: String,
    /// How many meetings carry it.
    pub meetings: u32,
}

/// A folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FolderView {
    /// Its ID; also accepted wherever a folder is named.
    #[schemars(with = "String")]
    pub id: FolderId,
    /// Its name.
    pub name: String,
    /// Its place in the tree, such as `Clients / Acme`.
    pub path: String,
    /// What it is for; may be empty.
    pub description: String,
    /// How many meetings sit directly in it.
    pub meetings: u32,
}

/// A synced calendar event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventView {
    /// Its ID.
    #[schemars(with = "String")]
    pub id: CalendarEventId,
    /// Its title.
    pub title: String,
    /// When it starts, in the user's zone.
    pub start: DateTime<FixedOffset>,
    /// When it ends.
    pub end: DateTime<FixedOffset>,
    /// True for an all-day event.
    pub all_day: bool,
    /// `zoom`, `google_meet`, `teams`, ... when it is a call.
    pub platform: Option<String>,
    /// The call's link.
    pub conference_url: Option<String>,
    /// The organiser's email.
    pub organizer: Option<String>,
    /// Invitees, by name or email.
    pub attendees: Vec<String>,
    /// The user's answer: `accepted`, `declined`, `tentative`, ...
    pub my_rsvp: Option<String>,
    /// True when someone outside the user's organisation is invited.
    pub external: bool,
}

/// Which calendar events to list: those starting on these local days.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventWindow {
    /// The first day, inclusive; `None` is a week ago.
    pub from: Option<NaiveDate>,
    /// The last day, inclusive; `None` is a week from today.
    pub to: Option<NaiveDate>,
}
