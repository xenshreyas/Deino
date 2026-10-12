//! [`NameSources`]: the names a transcript may contain, added to the
//! vocabulary automatically: everyone in People, every company, and the
//! invitees of the meeting's calendar event (ORG-01) who are not people yet.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::MeetingId;

use super::{KnownName, NameKind};
use crate::calendar::CalendarEventRepo;
use crate::companies::CompanyRepo;
use crate::meetings::MeetingLifecycleRepo;
use crate::people::PersonRepo;

/// Where the automatic names are read from.
#[derive(Clone)]
pub struct NameSources {
    /// People (merged ones are already left out).
    pub people: Arc<dyn PersonRepo>,
    /// Companies.
    pub companies: Arc<dyn CompanyRepo>,
    /// A meeting's calendar link.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// The linked calendar event and its invitees.
    pub calendar: Arc<dyn CalendarEventRepo>,
}

impl NameSources {
    /// Every known name, plus `meeting`'s invitees when there is a meeting.
    pub async fn names(&self, meeting: Option<&MeetingId>) -> Result<Vec<KnownName>, DbError> {
        let people = self.people.list().await?.into_iter().map(|p| p.name);
        let mut names: Vec<KnownName> = people.map(|text| named(text, NameKind::Person)).collect();
        let companies = self.companies.list().await?.into_iter().map(|c| c.name);
        names.extend(companies.map(|text| named(text, NameKind::Company)));
        if let Some(meeting) = meeting {
            let invitees = self.invitees(meeting).await?;
            names.extend(
                invitees
                    .into_iter()
                    .map(|text| named(text, NameKind::Person)),
            );
        }
        Ok(names)
    }

    /// The display names of `meeting`'s calendar invitees; an address
    /// standing in for a name is not one.
    async fn invitees(&self, meeting: &MeetingId) -> Result<Vec<String>, DbError> {
        let link = self
            .meetings
            .state(meeting)
            .await?
            .and_then(|s| s.calendar_event_id);
        let Some(event) = link else {
            return Ok(Vec::new());
        };
        let attendees = self
            .calendar
            .event(&event)
            .await?
            .map(|e| e.attendees)
            .unwrap_or_default();
        Ok(attendees
            .into_iter()
            .filter_map(|attendee| attendee.name)
            .filter(|name| !name.contains('@') && !name.trim().is_empty())
            .collect())
    }
}

fn named(text: String, kind: NameKind) -> KnownName {
    KnownName { text, kind }
}
