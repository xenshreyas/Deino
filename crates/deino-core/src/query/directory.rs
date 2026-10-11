//! [`DirectoryQueries`]: the people, companies, tags, folders and calendar
//! events a reader names meetings by.

use std::collections::HashMap;

use chrono::{Days, TimeDelta};
use deino_domain::{CompanyId, LocalZone, TagId};

use super::{
    CompanyView, EventView, EventWindow, FolderView, PersonView, QueryError, ReadPorts, TagView,
};
use crate::calendar::CalendarEvent;
use crate::people::Person;

/// How many people a list returns when the caller does not say.
pub const DEFAULT_PEOPLE: u32 = 100;
/// The most people one list returns.
pub const MAX_PEOPLE: u32 = 1_000;
/// How far around today the default event window reaches, in days.
const DEFAULT_EVENT_DAYS: u64 = 7;
/// The longest event window, in days.
pub const MAX_EVENT_DAYS: i64 = 92;
/// Separates folder names in a path.
const PATH_SEPARATOR: &str = " / ";

/// Lists the directory.
pub struct DirectoryQueries {
    ports: ReadPorts,
}

impl DirectoryQueries {
    /// The queries over `ports`.
    pub fn new(ports: ReadPorts) -> Self {
        Self { ports }
    }

    /// People whose name or email contains `text` (everyone without it), by
    /// name; at most `limit` (0 means [`DEFAULT_PEOPLE`]).
    pub async fn people(
        &self,
        text: Option<&str>,
        limit: u32,
    ) -> Result<Vec<PersonView>, QueryError> {
        let companies: HashMap<CompanyId, String> = self
            .ports
            .companies
            .list()
            .await?
            .into_iter()
            .map(|company| (company.id, company.name))
            .collect();
        let needle = text.map(str::to_lowercase);
        let limit = match limit {
            0 => DEFAULT_PEOPLE,
            limit => limit.min(MAX_PEOPLE),
        };
        let people = self.ports.meeting.people.list().await?;
        Ok(people
            .into_iter()
            .filter(|person| person.merged_into.is_none())
            .filter(|person| {
                needle
                    .as_deref()
                    .is_none_or(|needle| mentions(person, needle))
            })
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .map(|person| person_view(person, &companies))
            .collect())
    }

    /// Every company, with how many known people work there.
    pub async fn companies(&self) -> Result<Vec<CompanyView>, QueryError> {
        let mut staff: HashMap<CompanyId, u32> = HashMap::new();
        for person in self.ports.meeting.people.list().await? {
            if let Some(company) = person.company_id {
                *staff.entry(company).or_default() += 1;
            }
        }
        let companies = self.ports.companies.list().await?;
        Ok(companies
            .into_iter()
            .map(|company| CompanyView {
                people: staff.get(&company.id).copied().unwrap_or_default(),
                domain: company.domain.map(|domain| domain.as_str().to_owned()),
                name: company.name,
                id: company.id,
            })
            .collect())
    }

    /// Every tag, by name, with how many meetings carry it.
    pub async fn tags(&self) -> Result<Vec<TagView>, QueryError> {
        let mut uses: HashMap<TagId, u32> = HashMap::new();
        for link in self.ports.tagging.links().await? {
            *uses.entry(link.tag_id).or_default() += 1;
        }
        let tags = self.ports.tags.tags().await?;
        Ok(tags
            .into_iter()
            .map(|tag| TagView {
                meetings: uses.get(&tag.id).copied().unwrap_or_default(),
                id: tag.id,
                name: tag.name,
                description: tag.description,
            })
            .collect())
    }

    /// Every folder in tree order, each with its path.
    pub async fn folders(&self) -> Result<Vec<FolderView>, QueryError> {
        let tree = self.ports.folder_tree.tree().await?;
        let mut path: Vec<String> = Vec::new();
        let mut views = Vec::with_capacity(tree.entries.len());
        for entry in tree.entries {
            path.truncate(usize::try_from(entry.depth).unwrap_or(usize::MAX));
            path.push(entry.folder.name.clone());
            views.push(FolderView {
                id: entry.folder.id,
                name: entry.folder.name,
                path: path.join(PATH_SEPARATOR),
                description: entry.folder.description,
                meetings: entry.meeting_count,
            });
        }
        Ok(views)
    }

    /// Calendar events starting on the window's days, by start time.
    pub async fn events(&self, window: EventWindow) -> Result<Vec<EventView>, QueryError> {
        let zone = self.ports.meeting.zone.as_ref();
        let today = zone.day_of(self.ports.clock.now());
        let from = window
            .from
            .unwrap_or_else(|| today - Days::new(DEFAULT_EVENT_DAYS));
        let to = window
            .to
            .unwrap_or_else(|| today + Days::new(DEFAULT_EVENT_DAYS));
        let span = to.signed_duration_since(from);
        if span < TimeDelta::zero() || span > TimeDelta::days(MAX_EVENT_DAYS) {
            return Err(QueryError::InvalidRange);
        }
        let end = zone.start_of(to + Days::new(1));
        let events = self
            .ports
            .events
            .events_between(zone.start_of(from), end)
            .await?;
        Ok(events
            .into_iter()
            .map(|event| event_view(event, zone))
            .collect())
    }
}

fn mentions(person: &Person, needle: &str) -> bool {
    person.name.to_lowercase().contains(needle)
        || person
            .email
            .as_ref()
            .is_some_and(|email| email.as_str().to_lowercase().contains(needle))
}

fn person_view(person: Person, companies: &HashMap<CompanyId, String>) -> PersonView {
    PersonView {
        company: person
            .company_id
            .and_then(|company| companies.get(&company).cloned()),
        email: person.email.map(|email| email.as_str().to_owned()),
        id: person.id,
        name: person.name,
        title: person.title,
        is_me: person.is_me,
    }
}

fn event_view(event: CalendarEvent, zone: &dyn LocalZone) -> EventView {
    EventView {
        id: event.id,
        title: event.title,
        start: zone.local(event.start_at),
        end: zone.local(event.end_at),
        all_day: event.is_all_day,
        platform: event.platform.map(|platform| platform.as_str().to_owned()),
        conference_url: event.conference_url,
        organizer: event.organizer_email,
        attendees: event
            .attendees
            .into_iter()
            .filter_map(|attendee| attendee.name.or(attendee.email))
            .collect(),
        my_rsvp: event.my_rsvp.map(|rsvp| rsvp.as_str().to_owned()),
        external: event.is_external,
    }
}

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;
