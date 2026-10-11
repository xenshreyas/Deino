//! The upsert rules: how someone met in a meeting becomes a person, and
//! their email domain a company.
//!
//! Every source goes through [`EntityDirectory`]: calendar invitees (ORG-01's
//! `AttendeeDirectory`), named speakers (the `update_entities` stage, CAP-04)
//! and people added from an @mention. The rules:
//!
//! - **People are keyed by email** when there is one, else by their exact
//!   name (ignoring case), so naming the same speaker twice never makes two
//!   people. A key that belongs to a merged person resolves to the person it
//!   was merged into.
//! - **Companies are keyed by domain**, taken from a new person's address;
//!   personal mail providers (`gmail.com`) and personal vanity domains
//!   (`jordanellis.dev` for Jordan Ellis) never make one.
//! - **Upserts only add.** A person or company that exists is left exactly as
//!   the user (or a merge) left it.

use std::sync::Arc;

use deino_db::DbError;
use deino_domain::{Clock, CompanyId, Email, PersonId};

use super::names::{company_name, is_personal_domain, person_name};
use super::{PersonSeed, Upserted};
use crate::companies::{Company, CompanyDomain, CompanyRepo, is_public_email_domain};
use crate::people::{Person, PersonRepo};

/// How many `merged_into` links a lookup follows. Merges flatten chains, so
/// one is enough; the bound only guards against a corrupt cycle.
const MAX_MERGE_HOPS: usize = 8;

/// Adds people and companies by the module's rules.
pub struct EntityDirectory {
    people: Arc<dyn PersonRepo>,
    companies: Arc<dyn CompanyRepo>,
    clock: Arc<dyn Clock>,
}

impl EntityDirectory {
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
        }
    }

    /// The person `seed` names, added if new; `None` for a seed with neither
    /// an address nor a name.
    pub async fn upsert_person(&self, seed: &PersonSeed) -> Result<Option<Upserted>, DbError> {
        let name = seed
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty());
        match (&seed.email, name) {
            (Some(email), _) => self.upsert_by_email(email, name).await.map(Some),
            (None, Some(name)) => self.upsert_by_name(name).await.map(Some),
            (None, None) => Ok(None),
        }
    }

    /// The person with `email`, or the one they were merged into, added
    /// (with their company) if nobody has it.
    async fn upsert_by_email(
        &self,
        email: &Email,
        name: Option<&str>,
    ) -> Result<Upserted, DbError> {
        if let Some(found) = self.people.find_by_email(email).await? {
            return self.existing(found).await;
        }
        let name = name.map_or_else(|| person_name(email.as_str()), str::to_owned);
        let company_id = self.company_for(email, &name).await?;
        let mut person = self.new_person(name);
        person.email = Some(email.clone());
        person.company_id = company_id;
        match self.people.create(&person).await {
            Ok(()) => Ok(created(person.id)),
            // Another source added them a moment ago: theirs wins.
            Err(error) if error.is_unique_violation() => {
                match self.people.find_by_email(email).await? {
                    Some(found) => self.existing(found).await,
                    None => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    }

    /// The unmerged person called `name`, added if nobody is.
    async fn upsert_by_name(&self, name: &str) -> Result<Upserted, DbError> {
        if let Some(found) = self.people.find_by_name(name).await? {
            return self.existing(found).await;
        }
        let person = self.new_person(name.to_owned());
        self.people.create(&person).await?;
        Ok(created(person.id))
    }

    /// The company at `email`'s domain, added if new; `None` for a personal
    /// mail provider or `name`'s own vanity domain.
    pub async fn company_for(
        &self,
        email: &Email,
        name: &str,
    ) -> Result<Option<CompanyId>, DbError> {
        let domain = email.domain();
        if is_public_email_domain(domain) || is_personal_domain(domain, name) {
            return Ok(None);
        }
        let Some(domain) = CompanyDomain::parse(domain) else {
            return Ok(None);
        };
        self.upsert_company(&domain).await.map(Some)
    }

    /// The company with `domain`, or the one it was merged into, added if
    /// no company has the domain.
    pub async fn upsert_company(&self, domain: &CompanyDomain) -> Result<CompanyId, DbError> {
        if let Some(found) = self.companies.find_by_domain(domain).await? {
            return Ok(self.surviving_company(found).await?.id);
        }
        let company = Company::new(
            CompanyId::generate(self.clock.as_ref()),
            company_name(domain.as_str()),
            Some(domain.clone()),
            self.clock.now(),
        );
        match self.companies.create(&company).await {
            Ok(()) => Ok(company.id),
            Err(error) if error.is_unique_violation() => {
                match self.companies.find_by_domain(domain).await? {
                    Some(found) => Ok(self.surviving_company(found).await?.id),
                    None => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    }

    /// The person with `email` (or whoever they were merged into), without
    /// adding anyone; `None` when nobody has the address.
    pub async fn person_with_email(&self, email: &Email) -> Result<Option<PersonId>, DbError> {
        let Some(found) = self.people.find_by_email(email).await? else {
            return Ok(None);
        };
        Ok(Some(self.surviving_person(found).await?.id))
    }

    /// `person`, or whoever they were merged into.
    pub async fn surviving_person(&self, mut person: Person) -> Result<Person, DbError> {
        for _ in 0..MAX_MERGE_HOPS {
            let Some(target) = person.merged_into else {
                break;
            };
            match self.people.find(&target).await? {
                Some(next) => person = next,
                None => break,
            }
        }
        Ok(person)
    }

    /// `company`, or whichever it was merged into.
    pub async fn surviving_company(&self, mut company: Company) -> Result<Company, DbError> {
        for _ in 0..MAX_MERGE_HOPS {
            let Some(target) = company.merged_into else {
                break;
            };
            match self.companies.find(&target).await? {
                Some(next) => company = next,
                None => break,
            }
        }
        Ok(company)
    }

    async fn existing(&self, person: Person) -> Result<Upserted, DbError> {
        let survivor = self.surviving_person(person).await?;
        Ok(Upserted {
            id: survivor.id,
            created: false,
        })
    }

    /// A person called `name` with nothing else known yet.
    fn new_person(&self, name: String) -> Person {
        let now = self.clock.now();
        Person {
            id: PersonId::generate(self.clock.as_ref()),
            name,
            email: None,
            company_id: None,
            title: None,
            avatar_path: None,
            notes_md: String::new(),
            summary_md: None,
            summary_updated_at: None,
            is_me: false,
            merged_into: None,
            created_at: now,
            updated_at: now,
        }
    }
}

fn created(id: PersonId) -> Upserted {
    Upserted { id, created: true }
}

#[cfg(test)]
mod directory_tests;
