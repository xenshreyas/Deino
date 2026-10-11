//! People and companies from calendar invitations (ORG-01), through the
//! entity rules (ORG-04).
//!
//! [`AttendeeDirectory`] decides which invitees count as people the user
//! meets and hands each to [`EntityDirectory`], which owns the upsert rules
//! (keyed by email, companies from the email's domain, never a personal
//! mail provider's, never changing someone who exists). Large broadcast
//! invitations, declined events and declined invitees add nobody, since
//! none of them say the user actually meets these people.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deino_db::DbError;
use deino_domain::{Clock, Email, PersonId, Rsvp};

use super::EventAttendee;
use super::batch::SyncedEvent;
use crate::companies::CompanyRepo;
use crate::entities::{EntityDirectory, PersonSeed};
use crate::people::PersonRepo;
use crate::profile::InternalDomains;

/// Invitations with more attendees than this are broadcasts (all-hands,
/// webinars), not people the user meets; they add nobody.
pub const MAX_ATTENDEES_FOR_PEOPLE: usize = 25;

/// Adds the people and companies of calendar invitations.
pub struct AttendeeDirectory {
    entities: Arc<EntityDirectory>,
    /// Addresses already looked up this session, so a sync every two
    /// minutes does not read every attendee again.
    known: Mutex<BTreeSet<Email>>,
}

impl AttendeeDirectory {
    /// A directory adding through the entity rules.
    pub fn new(entities: Arc<EntityDirectory>) -> Self {
        Self {
            entities,
            known: Mutex::new(BTreeSet::new()),
        }
    }

    /// A directory over its own [`EntityDirectory`] on these repositories.
    pub fn over(
        people: Arc<dyn PersonRepo>,
        companies: Arc<dyn CompanyRepo>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self::new(Arc::new(EntityDirectory::new(people, companies, clock)))
    }

    /// Adds every new person invited to `events` (and their company);
    /// returns how many people were added.
    pub async fn record<'a>(
        &self,
        events: impl IntoIterator<Item = &'a SyncedEvent>,
        own: &InternalDomains,
    ) -> Result<usize, DbError> {
        let invitees: BTreeSet<(Email, Option<String>)> = events
            .into_iter()
            .filter(|event| meets_people(event))
            .flat_map(|event| &event.attendees)
            .filter_map(invitee)
            .filter(|(email, _)| !own.is_own(email))
            .collect();
        let mut added = 0;
        for (email, name) in invitees {
            if self.known().contains(&email) {
                continue;
            }
            let seed = PersonSeed {
                email: Some(email.clone()),
                name,
            };
            let upserted = self.entities.upsert_person(&seed).await?;
            added += usize::from(upserted.is_some_and(|upserted| upserted.created));
            self.known().insert(email);
        }
        Ok(added)
    }

    /// The addresses already looked up this session.
    fn known(&self) -> MutexGuard<'_, BTreeSet<Email>> {
        self.known.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The person with `email` (whoever they were merged into), or, with
    /// `create`, a new person (and their company) through the entity rules;
    /// `None` when nobody has the address and `create` is false. Event
    /// linking (ORG-03) turns invitees into attendees with it.
    pub(super) async fn person_for(
        &self,
        email: &Email,
        name: Option<String>,
        create: bool,
    ) -> Result<Option<PersonId>, DbError> {
        if !create {
            return self.entities.person_with_email(email).await;
        }
        let seed = PersonSeed {
            email: Some(email.clone()),
            name,
        };
        Ok(self.entities.upsert_person(&seed).await?.map(|upserted| upserted.id))
    }
}

/// True when the user takes part in `event` and it is small enough to say
/// they meet its attendees.
fn meets_people(event: &SyncedEvent) -> bool {
    event.my_rsvp != Some(Rsvp::Declined) && event.attendees.len() <= MAX_ATTENDEES_FOR_PEOPLE
}

/// An attendee who did not decline, as an address and a display name.
pub(super) fn invitee(attendee: &EventAttendee) -> Option<(Email, Option<String>)> {
    if attendee.rsvp == Some(Rsvp::Declined) {
        return None;
    }
    let email = Email::parse(attendee.email.as_deref()?).ok()?;
    let name = attendee.name.clone().filter(|name| !name.contains('@'));
    Some((email, name))
}

#[cfg(test)]
mod entities_tests;
