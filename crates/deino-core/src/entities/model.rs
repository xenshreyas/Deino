//! What the People and Companies pages show, and what the user types.

use chrono::{DateTime, Utc};
use deino_domain::{ActionItemId, CalendarEventId, CompanyId, Email, MeetingId, PersonId};
use serde::{Deserialize, Serialize};

use crate::companies::{Company, CompanyDomain};
use crate::people::Person;

/// A row of the People list: who, where they work, and when the user last
/// met them. The aggregates are computed on read (ADR 0030).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PersonRow {
    /// The person.
    pub id: PersonId,
    /// Their name.
    pub name: String,
    /// Their email address.
    pub email: Option<Email>,
    /// Their job title.
    pub title: Option<String>,
    /// Where they work.
    pub company_id: Option<CompanyId>,
    /// The company's name, for the row's second line.
    pub company_name: Option<String>,
    /// The company's cached logo, shown on the person's avatar.
    pub company_logo_path: Option<String>,
    /// When their latest meeting outside the trash happened.
    pub last_met_at: Option<DateTime<Utc>>,
    /// How many meetings outside the trash they attended.
    pub meeting_count: u32,
}

/// A row of the Companies list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompanyRow {
    /// The company.
    pub id: CompanyId,
    /// Its name.
    pub name: String,
    /// Its web domain.
    pub domain: Option<CompanyDomain>,
    /// Its cached logo.
    pub logo_path: Option<String>,
    /// How many listed people work there.
    pub people_count: u32,
    /// How many meetings outside the trash any of them attended.
    pub meeting_count: u32,
    /// When the latest of those meetings happened.
    pub last_met_at: Option<DateTime<Utc>>,
}

/// A meeting on an entity page or hover card.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EntityMeeting {
    /// The meeting.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it started, or was created when the start is unknown.
    pub at: DateTime<Utc>,
    /// Its recorded length in seconds, when known.
    pub duration_s: Option<u32>,
}

/// An open action item on an entity page. Its own shape, so the page does
/// not change when the action items feature (AI-05) grows its model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EntityActionItem {
    /// The item.
    pub id: ActionItemId,
    /// What to do.
    pub title: String,
    /// When it is due.
    pub due_at: Option<DateTime<Utc>>,
    /// Who should do it.
    pub assignee_person_id: Option<PersonId>,
    /// The meeting it came from.
    pub meeting_id: Option<MeetingId>,
}

/// A calendar event coming up with the person or company.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingEvent {
    /// The event.
    pub id: CalendarEventId,
    /// Its title; may be empty.
    pub title: String,
    /// When it starts.
    pub start_at: DateTime<Utc>,
    /// When it ends.
    pub end_at: DateTime<Utc>,
    /// Its video-call link.
    pub conference_url: Option<String>,
}

/// Everything the person page shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PersonPage {
    /// The person (the survivor, when the asked-for person was merged).
    pub person: Person,
    /// Where they work.
    pub company: Option<Company>,
    /// Their meetings, most recent first.
    pub meetings: Vec<EntityMeeting>,
    /// Their open action items, soonest due first.
    pub action_items: Vec<EntityActionItem>,
    /// Calendar events with them in the next two weeks.
    pub upcoming: Vec<UpcomingEvent>,
}

/// Everything the company page shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompanyPage {
    /// The company (the survivor, when the asked-for one was merged).
    pub company: Company,
    /// Its people, most recently met first.
    pub people: Vec<PersonRow>,
    /// Meetings with any of them, most recent first.
    pub meetings: Vec<EntityMeeting>,
    /// Their open action items, soonest due first.
    pub action_items: Vec<EntityActionItem>,
    /// Calendar events with them in the next two weeks.
    pub upcoming: Vec<UpcomingEvent>,
}

/// A person's details as the user types them (add and edit).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PersonFields {
    /// Their name; required.
    pub name: String,
    /// Their email address; blank for none.
    pub email: Option<String>,
    /// Their job title; blank for none.
    pub title: Option<String>,
    /// Where they work.
    pub company_id: Option<CompanyId>,
}

/// A company's details as the user types them (add and edit).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompanyFields {
    /// Its name; required.
    pub name: String,
    /// Its web domain (`acme.com`, or a URL or address at it); blank for none.
    pub domain: Option<String>,
}

/// Someone met in a meeting or named in notes, before they are a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonSeed {
    /// Their address; the upsert key when present.
    pub email: Option<Email>,
    /// Their name as the source wrote it; the key when there is no address.
    pub name: Option<String>,
}

/// The outcome of an upsert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Upserted {
    /// The person, or the person a matching one was merged into.
    pub id: PersonId,
    /// True when the person was created by this upsert.
    pub created: bool,
}
