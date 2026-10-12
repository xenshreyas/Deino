//! [`CompanyService`]: the Companies list and page, adding, editing and
//! merging companies.

use deino_domain::{ActionItemStatus, CompanyId};

use super::person_service::entity_item;
use super::{
    CompanyFields, CompanyMerge, CompanyPage, CompanyRow, EntityActionItem, EntityError,
    EntityPorts, PersonRow, fields, upcoming,
};
use crate::companies::Company;

/// Meetings listed on a company page.
pub const COMPANY_PAGE_MEETINGS: u32 = 50;

/// The rules behind the Companies pages.
pub struct CompanyService {
    ports: EntityPorts,
}

impl CompanyService {
    /// Creates the service over its ports.
    pub fn new(ports: EntityPorts) -> Self {
        Self { ports }
    }

    /// Every company, most recently met first.
    pub async fn list(&self) -> Result<Vec<CompanyRow>, EntityError> {
        Ok(self.ports.reads.company_rows().await?)
    }

    /// The page of `id`, or of the company it was merged into.
    pub async fn page(&self, id: &CompanyId) -> Result<CompanyPage, EntityError> {
        let company = self.surviving(id).await?;
        let people = self.ports.reads.person_rows(Some(&company.id)).await?;
        let meetings = self
            .ports
            .reads
            .company_meetings(&company.id, COMPANY_PAGE_MEETINGS)
            .await?;
        let action_items = self.open_items(&people).await?;
        let (from, to) = upcoming::window(self.ports.clock.now());
        let events = self.ports.events.events_between(from, to).await?;
        let domain = company.domain.clone();
        let upcoming = upcoming::upcoming_with(&events, |email| {
            domain
                .as_ref()
                .is_some_and(|domain| email.domain() == domain.as_str())
                || people
                    .iter()
                    .any(|person| person.email.as_ref() == Some(email))
        });
        Ok(CompanyPage {
            company,
            people,
            meetings,
            action_items,
            upcoming,
        })
    }

    /// Adds a company from the form. A domain another company has is
    /// refused, so the user merges instead of duplicating.
    pub async fn create(&self, input: &CompanyFields) -> Result<Company, EntityError> {
        let company = Company::new(
            CompanyId::generate(self.ports.clock.as_ref()),
            fields::name(&input.name)?,
            fields::domain(input.domain.as_deref())?,
            self.ports.clock.now(),
        );
        self.ports
            .companies
            .create(&company)
            .await
            .map_err(EntityError::unique_or(EntityError::DomainTaken))?;
        Ok(company)
    }

    /// Saves the form's fields on `id`. A new domain forgets the old
    /// domain's logo record, so the new site is asked once.
    pub async fn update(
        &self,
        id: &CompanyId,
        input: &CompanyFields,
    ) -> Result<Company, EntityError> {
        let mut company = self.listed(id).await?;
        let domain = fields::domain(input.domain.as_deref())?;
        if domain != company.domain {
            company.logo_checked_at = None;
        }
        company.name = fields::name(&input.name)?;
        company.domain = domain;
        company.updated_at = self.ports.clock.now();
        self.ports
            .companies
            .update(&company)
            .await
            .map_err(EntityError::unique_or(EntityError::DomainTaken))?;
        Ok(company)
    }

    /// Saves the user's notes about `id`.
    pub async fn set_notes(&self, id: &CompanyId, notes: &str) -> Result<(), EntityError> {
        let now = self.ports.clock.now();
        if self.ports.writes.set_company_notes(id, notes, now).await? {
            Ok(())
        } else {
            Err(EntityError::CompanyNotFound(*id))
        }
    }

    /// Merges `absorbed` into `survivor`, moving its people in one
    /// transaction (see [`super::merge`]); returns the survivor.
    pub async fn merge(
        &self,
        survivor: &CompanyId,
        absorbed: &CompanyId,
    ) -> Result<Company, EntityError> {
        if survivor == absorbed {
            return Err(EntityError::SameEntity);
        }
        let kept = self.listed(survivor).await?;
        let gone = self.listed(absorbed).await?;
        let plan = CompanyMerge::plan(kept, gone, self.ports.clock.now());
        self.ports.merger.merge_companies(&plan).await?;
        log::info!(target: "entities", "merged company {absorbed} into {survivor}");
        Ok(plan.survivor)
    }

    /// `id`, or whichever company it was merged into.
    async fn surviving(&self, id: &CompanyId) -> Result<Company, EntityError> {
        let company = self.ports.companies.find(id).await?;
        let company = company.ok_or(EntityError::CompanyNotFound(*id))?;
        Ok(self.ports.directory.surviving_company(company).await?)
    }

    /// `id`, which must exist and not be merged away.
    async fn listed(&self, id: &CompanyId) -> Result<Company, EntityError> {
        let company = self.ports.companies.find(id).await?;
        company
            .filter(|company| company.merged_into.is_none())
            .ok_or(EntityError::CompanyNotFound(*id))
    }

    /// The open action items of the company's people, soonest due first.
    async fn open_items(&self, people: &[PersonRow]) -> Result<Vec<EntityActionItem>, EntityError> {
        let mut items = Vec::new();
        for person in people {
            let open = self
                .ports
                .action_items
                .list_for_assignee(&person.id, ActionItemStatus::Open)
                .await?;
            items.extend(open.into_iter().map(entity_item));
        }
        // Undated items last, like the global list.
        items.sort_by_key(|item| (item.due_at.is_none(), item.due_at, item.id));
        Ok(items)
    }
}

#[cfg(test)]
mod company_service_tests;
