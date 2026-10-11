//! People and companies from calendar attendees (ORG-01 seeds what ORG-04
//! builds on).
//!
//! [`AttendeeDirectory`] adds each person the user is invited to meet,
//! keyed by email, and their company, keyed by the email's domain (never a
//! personal mail provider's). It only ever adds: a person or company that
//! exists is left exactly as the user (or ORG-04) left it. Large broadcast
//! invitations, declined events and declined invitees add nobody, since none
//! of them say the user actually meets these people.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deino_db::DbError;
use deino_domain::{Clock, CompanyId, Email, PersonId, Rsvp};

use super::EventAttendee;
use super::batch::SyncedEvent;
use super::names::{company_name, person_name};
use crate::companies::{Company, CompanyDomain, CompanyRepo, is_public_email_domain};
use crate::people::{Person, PersonRepo};
use crate::profile::InternalDomains;

/// Invitations with more attendees than this are broadcasts (all-hands,
/// webinars), not people the user meets; they add nobody.
pub const MAX_ATTENDEES_FOR_PEOPLE: usize = 25;

/// Adds the people and companies of calendar invitations.
pub struct AttendeeDirectory {
    people: Arc<dyn PersonRepo>,
    companies: Arc<dyn CompanyRepo>,
    clock: Arc<dyn Clock>,
    /// Addresses already looked up this session, so a sync every two
    /// minutes does not read every attendee again.
    known: Mutex<BTreeSet<Email>>,
}

impl AttendeeDirectory {
    /// A directory over the people and company repositories.
    pub fn new(
        people: Arc<dyn PersonRepo>,
        companies: Arc<dyn CompanyRepo>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            people,
            companies,
            clock,
            known: Mutex::new(BTreeSet::new()),
        }
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
            added += usize::from(self.add_if_new(&email, name).await?);
            self.known().insert(email);
        }
        Ok(added)
    }

    /// The addresses already looked up this session.
    fn known(&self) -> MutexGuard<'_, BTreeSet<Email>> {
        self.known.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Adds the person with `email` unless someone has it; true when added.
    async fn add_if_new(&self, email: &Email, name: Option<String>) -> Result<bool, DbError> {
        if self.people.find_by_email(email).await?.is_some() {
            return Ok(false);
        }
        let now = self.clock.now();
        let person = Person {
            id: PersonId::generate(self.clock.as_ref()),
            name: name.unwrap_or_else(|| person_name(email.as_str())),
            email: Some(email.clone()),
            company_id: self.company_for(email).await?,
            title: None,
            avatar_path: None,
            notes_md: String::new(),
            summary_md: None,
            summary_updated_at: None,
            is_me: false,
            merged_into: None,
            created_at: now,
            updated_at: now,
        };
        self.people.create(&person).await?;
        Ok(true)
    }

    /// The company at `email`'s domain, added if new; `None` for a personal
    /// mail provider.
    async fn company_for(&self, email: &Email) -> Result<Option<CompanyId>, DbError> {
        let domain = email.domain();
        let Some(domain) = CompanyDomain::parse(domain).filter(|_| !is_public_email_domain(domain))
        else {
            return Ok(None);
        };
        if let Some(company) = self.companies.find_by_domain(&domain).await? {
            return Ok(Some(company.id));
        }
        let now = self.clock.now();
        let company = Company {
            id: CompanyId::generate(self.clock.as_ref()),
            name: company_name(domain.as_str()),
            domain: Some(domain),
            logo_path: None,
            notes_md: String::new(),
            summary_md: None,
            created_at: now,
            updated_at: now,
        };
        self.companies.create(&company).await?;
        Ok(Some(company.id))
    }
}

/// True when the user takes part in `event` and it is small enough to say
/// they meet its attendees.
fn meets_people(event: &SyncedEvent) -> bool {
    event.my_rsvp != Some(Rsvp::Declined) && event.attendees.len() <= MAX_ATTENDEES_FOR_PEOPLE
}

/// An attendee who did not decline, as an address and a display name.
fn invitee(attendee: &EventAttendee) -> Option<(Email, Option<String>)> {
    if attendee.rsvp == Some(Rsvp::Declined) {
        return None;
    }
    let email = Email::parse(attendee.email.as_deref()?).ok()?;
    let name = attendee.name.clone().filter(|name| !name.contains('@'));
    Some((email, name))
}

#[cfg(test)]
mod entities_tests;
