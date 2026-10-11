//! [`AttendanceService`]: who was in a meeting, for the meeting header's
//! attendee chips (FND-10 left them for ORG-04), and adding or removing
//! someone by hand.

use deino_domain::{AttendeeRole, CompanyId, Email, MeetingId, PersonId};
use serde::Serialize;

use super::{EntityError, EntityPorts};
use crate::people::Attendee;

/// A person in a meeting, as the header shows them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPerson {
    /// The person (the survivor, if the attendee was merged since).
    pub person_id: PersonId,
    /// Their name.
    pub name: String,
    /// Their address.
    pub email: Option<Email>,
    /// Their job title.
    pub title: Option<String>,
    /// Where they work.
    pub company_id: Option<CompanyId>,
    /// The company's cached logo.
    pub company_logo_path: Option<String>,
    /// How they took part.
    pub role: AttendeeRole,
    /// True for the user.
    pub is_me: bool,
}

/// Reads and edits who attended a meeting.
pub struct AttendanceService {
    ports: EntityPorts,
}

impl AttendanceService {
    /// Creates the service over its ports.
    pub fn new(ports: EntityPorts) -> Self {
        Self { ports }
    }

    /// The meeting's attendees: the organiser first, then in the order they
    /// were added, the user last.
    pub async fn attendees(&self, meeting: &MeetingId) -> Result<Vec<MeetingPerson>, EntityError> {
        let mut people = Vec::new();
        for attendee in self.ports.attendees.attendees(meeting).await? {
            if let Some(person) = self.meeting_person(attendee).await? {
                people.push(person);
            }
        }
        people.sort_by_key(|person| (person.is_me, person.role != AttendeeRole::Organizer));
        Ok(people)
    }

    /// Adds `person` to the meeting as an invitee; someone already on it
    /// keeps their role.
    pub async fn add(&self, meeting: &MeetingId, person: &PersonId) -> Result<(), EntityError> {
        let found = self.ports.people.find(person).await?;
        let found = found.ok_or(EntityError::PersonNotFound(*person))?;
        let survivor = self.ports.directory.surviving_person(found).await?;
        let already = self.ports.attendees.attendees(meeting).await?;
        if already
            .iter()
            .any(|attendee| attendee.person_id == survivor.id)
        {
            return Ok(());
        }
        let attendee = Attendee {
            meeting_id: meeting.clone(),
            person_id: survivor.id,
            role: AttendeeRole::Invitee,
            rsvp: None,
        };
        Ok(self.ports.attendees.upsert_attendee(&attendee).await?)
    }

    /// Takes `person` off the meeting; true when they were on it.
    pub async fn remove(
        &self,
        meeting: &MeetingId,
        person: &PersonId,
    ) -> Result<bool, EntityError> {
        Ok(self
            .ports
            .attendees
            .remove_attendee(meeting, person)
            .await?)
    }

    async fn meeting_person(
        &self,
        attendee: Attendee,
    ) -> Result<Option<MeetingPerson>, EntityError> {
        let Some(person) = self.ports.people.find(&attendee.person_id).await? else {
            return Ok(None);
        };
        let person = self.ports.directory.surviving_person(person).await?;
        let logo = match person.company_id {
            Some(company) => self
                .ports
                .companies
                .find(&company)
                .await?
                .and_then(|company| company.logo_path),
            None => None,
        };
        Ok(Some(MeetingPerson {
            person_id: person.id,
            name: person.name,
            email: person.email,
            title: person.title,
            company_id: person.company_id,
            company_logo_path: logo,
            role: attendee.role,
            is_me: person.is_me,
        }))
    }
}
