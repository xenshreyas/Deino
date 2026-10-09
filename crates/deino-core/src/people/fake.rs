//! An in-memory store for the person and attendee ports.

use std::sync::Mutex;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{CompanyId, Email, MeetingId, PersonId};

use super::{Attendee, AttendeeRepo, Person, PersonRepo};
use crate::meetings::Meeting;
use crate::testing::lock;

/// Keeps people and attendees in `Vec`s and honours the [`PersonRepo`] and
/// [`AttendeeRepo`] contracts, including their orderings and hiding merged
/// people from lists. `meetings_of` answers from the meetings given to
/// [`FakePeopleRepo::with_meetings`], all of them outside the trash.
#[derive(Debug, Default)]
pub struct FakePeopleRepo {
    people: Mutex<Vec<Person>>,
    attendees: Mutex<Vec<Attendee>>,
    meetings: Vec<Meeting>,
}

impl FakePeopleRepo {
    /// A repository that knows `meetings`, for `meetings_of`.
    pub fn with_meetings(meetings: impl IntoIterator<Item = Meeting>) -> Self {
        Self {
            meetings: meetings.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Unmerged people passing `keep`, by name.
    fn listed(&self, keep: impl Fn(&Person) -> bool) -> Vec<Person> {
        let people = lock(&self.people);
        let mut listed: Vec<Person> = people
            .iter()
            .filter(|person| person.merged_into.is_none() && keep(person))
            .cloned()
            .collect();
        listed.sort_by_key(|person| (person.name.to_lowercase(), person.id));
        listed
    }

    /// Every stored person, merged or not, in insertion order.
    pub(super) fn people(&self) -> Vec<Person> {
        lock(&self.people).clone()
    }

    /// Changes the stored people under one lock, like a transaction.
    pub(super) fn update_people<T>(&self, change: impl FnOnce(&mut Vec<Person>) -> T) -> T {
        change(&mut lock(&self.people))
    }
}

#[async_trait]
impl PersonRepo for FakePeopleRepo {
    async fn create(&self, person: &Person) -> Result<(), DbError> {
        lock(&self.people).push(person.clone());
        Ok(())
    }

    async fn find(&self, id: &PersonId) -> Result<Option<Person>, DbError> {
        Ok(lock(&self.people)
            .iter()
            .find(|person| person.id == *id)
            .cloned())
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<Person>, DbError> {
        let people = lock(&self.people);
        Ok(people
            .iter()
            .find(|person| person.email.as_ref() == Some(email))
            .cloned())
    }

    async fn list(&self) -> Result<Vec<Person>, DbError> {
        Ok(self.listed(|_| true))
    }

    async fn list_by_company(&self, company: &CompanyId) -> Result<Vec<Person>, DbError> {
        Ok(self.listed(|person| person.company_id == Some(*company)))
    }

    async fn search(&self, prefix: &str, limit: u32) -> Result<Vec<Person>, DbError> {
        let prefix = prefix.to_ascii_lowercase();
        let matches = |text: &str| {
            let text = text.to_ascii_lowercase();
            text.starts_with(&prefix) || text.contains(&format!(" {prefix}"))
        };
        let mut found = self.listed(|person| {
            matches(&person.name)
                || person
                    .email
                    .as_ref()
                    .is_some_and(|email| email.as_str().to_ascii_lowercase().starts_with(&prefix))
        });
        found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        Ok(found)
    }

    async fn update(&self, person: &Person) -> Result<bool, DbError> {
        let mut people = lock(&self.people);
        let stored = people.iter_mut().find(|stored| stored.id == person.id);
        Ok(stored.map(|stored| *stored = person.clone()).is_some())
    }

    async fn delete(&self, id: &PersonId) -> Result<bool, DbError> {
        let mut people = lock(&self.people);
        let before = people.len();
        people.retain(|person| person.id != *id);
        lock(&self.attendees).retain(|attendee| attendee.person_id != *id);
        Ok(people.len() < before)
    }
}

#[async_trait]
impl AttendeeRepo for FakePeopleRepo {
    async fn upsert_attendee(&self, attendee: &Attendee) -> Result<(), DbError> {
        let mut attendees = lock(&self.attendees);
        let existing = attendees.iter_mut().find(|stored| {
            stored.meeting_id == attendee.meeting_id && stored.person_id == attendee.person_id
        });
        match existing {
            Some(stored) => *stored = attendee.clone(),
            None => attendees.push(attendee.clone()),
        }
        Ok(())
    }

    async fn remove_attendee(
        &self,
        meeting: &MeetingId,
        person: &PersonId,
    ) -> Result<bool, DbError> {
        let mut attendees = lock(&self.attendees);
        let before = attendees.len();
        attendees.retain(|stored| !(stored.meeting_id == *meeting && stored.person_id == *person));
        Ok(attendees.len() < before)
    }

    async fn attendees(&self, meeting: &MeetingId) -> Result<Vec<Attendee>, DbError> {
        let attendees = lock(&self.attendees);
        Ok(attendees
            .iter()
            .filter(|stored| stored.meeting_id == *meeting)
            .cloned()
            .collect())
    }

    async fn meetings_of(&self, person: &PersonId) -> Result<Vec<Meeting>, DbError> {
        let attendees = lock(&self.attendees);
        let attended = |meeting: &&Meeting| {
            attendees
                .iter()
                .any(|a| a.person_id == *person && a.meeting_id == meeting.id)
        };
        let mut meetings: Vec<Meeting> = self.meetings.iter().filter(attended).cloned().collect();
        meetings.sort_by(|a, b| (&b.created_at, &b.id).cmp(&(&a.created_at, &a.id)));
        Ok(meetings)
    }
}
