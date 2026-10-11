//! [`PersonService`]: the People list and page, adding, editing and merging
//! people.

use deino_domain::{ActionItemStatus, Email, PersonId};

use super::{
    EntityActionItem, EntityError, EntityPorts, PersonFields, PersonMerge, PersonPage, PersonRow,
    PersonSeed, fields, upcoming,
};
use crate::action_items::ActionItem;
use crate::people::Person;

/// Meetings listed on a person page.
pub const PAGE_MEETINGS: u32 = 50;

/// The rules behind the People pages.
pub struct PersonService {
    ports: EntityPorts,
}

impl PersonService {
    /// Creates the service over its ports.
    pub fn new(ports: EntityPorts) -> Self {
        Self { ports }
    }

    /// Everyone the user meets, most recently met first.
    pub async fn list(&self) -> Result<Vec<PersonRow>, EntityError> {
        Ok(self.ports.reads.person_rows(None).await?)
    }

    /// The page of `id`, or of the person they were merged into.
    pub async fn page(&self, id: &PersonId) -> Result<PersonPage, EntityError> {
        let person = self.surviving(id).await?;
        let company = match person.company_id {
            Some(company) => self.ports.companies.find(&company).await?,
            None => None,
        };
        let meetings = self
            .ports
            .reads
            .person_meetings(&person.id, PAGE_MEETINGS)
            .await?;
        let action_items = self.open_items(&person.id).await?;
        let upcoming = self.upcoming(person.email.as_ref()).await?;
        Ok(PersonPage {
            person,
            company,
            meetings,
            action_items,
            upcoming,
        })
    }

    /// Adds a person from the form. An address someone already has is
    /// refused, so the user merges instead of duplicating.
    pub async fn create(&self, input: &PersonFields) -> Result<Person, EntityError> {
        let name = fields::name(&input.name)?;
        let email = fields::email(input.email.as_deref())?;
        let company_id = match (&email, input.company_id) {
            (_, Some(company)) => Some(self.listed_company(&company).await?),
            (Some(email), None) => self.ports.directory.company_for(email, &name).await?,
            (None, None) => None,
        };
        let now = self.ports.clock.now();
        let person = Person {
            id: PersonId::generate(self.ports.clock.as_ref()),
            name,
            email,
            company_id,
            title: fields::text(input.title.as_deref()),
            avatar_path: None,
            notes_md: String::new(),
            summary_md: None,
            summary_updated_at: None,
            is_me: false,
            merged_into: None,
            created_at: now,
            updated_at: now,
        };
        self.ports
            .people
            .create(&person)
            .await
            .map_err(EntityError::unique_or(EntityError::EmailTaken))?;
        Ok(person)
    }

    /// The person called `name` (an @mention of someone new), added if
    /// nobody is; an existing namesake is reused, never duplicated.
    pub async fn from_mention(&self, name: &str) -> Result<Person, EntityError> {
        let seed = PersonSeed {
            email: None,
            name: Some(fields::name(name)?),
        };
        let upserted = self.ports.directory.upsert_person(&seed).await?;
        let id = upserted.ok_or(EntityError::EmptyName)?.id;
        self.surviving(&id).await
    }

    /// Saves the form's fields on `id`.
    pub async fn update(&self, id: &PersonId, input: &PersonFields) -> Result<Person, EntityError> {
        let mut person = self.listed(id).await?;
        person.name = fields::name(&input.name)?;
        person.email = fields::email(input.email.as_deref())?;
        person.title = fields::text(input.title.as_deref());
        person.company_id = match input.company_id {
            Some(company) => Some(self.listed_company(&company).await?),
            None => None,
        };
        person.updated_at = self.ports.clock.now();
        self.ports
            .people
            .update(&person)
            .await
            .map_err(EntityError::unique_or(EntityError::EmailTaken))?;
        Ok(person)
    }

    /// Saves the user's notes about `id`.
    pub async fn set_notes(&self, id: &PersonId, notes: &str) -> Result<(), EntityError> {
        let now = self.ports.clock.now();
        if self.ports.writes.set_person_notes(id, notes, now).await? {
            Ok(())
        } else {
            Err(EntityError::PersonNotFound(*id))
        }
    }

    /// Merges `absorbed` into `survivor`, moving every link in one
    /// transaction (see [`super::merge`]); returns the survivor.
    pub async fn merge(
        &self,
        survivor: &PersonId,
        absorbed: &PersonId,
    ) -> Result<Person, EntityError> {
        if survivor == absorbed {
            return Err(EntityError::SameEntity);
        }
        let kept = self.listed(survivor).await?;
        let gone = self.listed(absorbed).await?;
        let plan = PersonMerge::plan(kept, gone, self.ports.clock.now());
        self.ports.merger.merge_people(&plan).await?;
        log::info!(target: "entities", "merged person {absorbed} into {survivor}");
        Ok(plan.survivor)
    }

    /// `id`, or whoever it was merged into.
    pub(super) async fn surviving(&self, id: &PersonId) -> Result<Person, EntityError> {
        let person = self.ports.people.find(id).await?;
        let person = person.ok_or(EntityError::PersonNotFound(*id))?;
        Ok(self.ports.directory.surviving_person(person).await?)
    }

    /// `id`, which must exist and not be merged away.
    async fn listed(&self, id: &PersonId) -> Result<Person, EntityError> {
        let person = self.ports.people.find(id).await?;
        person
            .filter(|person| person.merged_into.is_none())
            .ok_or(EntityError::PersonNotFound(*id))
    }

    /// The company a form named, or the one it was merged into.
    async fn listed_company(
        &self,
        id: &deino_domain::CompanyId,
    ) -> Result<deino_domain::CompanyId, EntityError> {
        let company = self.ports.companies.find(id).await?;
        let company = company.ok_or(EntityError::CompanyNotFound(*id))?;
        Ok(self.ports.directory.surviving_company(company).await?.id)
    }

    /// The person's open action items, soonest due first.
    async fn open_items(&self, id: &PersonId) -> Result<Vec<EntityActionItem>, EntityError> {
        let items = self
            .ports
            .action_items
            .list_for_assignee(id, ActionItemStatus::Open)
            .await?;
        Ok(items.into_iter().map(entity_item).collect())
    }

    /// Calendar events in the next two weeks that invite `email`.
    async fn upcoming(
        &self,
        email: Option<&Email>,
    ) -> Result<Vec<super::UpcomingEvent>, EntityError> {
        let Some(email) = email else {
            return Ok(Vec::new());
        };
        let (from, to) = upcoming::window(self.ports.clock.now());
        let events = self.ports.events.events_between(from, to).await?;
        Ok(upcoming::upcoming_with(&events, |invited| invited == email))
    }
}

/// An action item as an entity page shows it.
pub(super) fn entity_item(item: ActionItem) -> EntityActionItem {
    EntityActionItem {
        id: item.id,
        title: item.title,
        due_at: item.due_at,
        assignee_person_id: item.assignee_person_id,
        meeting_id: item.meeting_id,
    }
}

#[cfg(test)]
mod merge_tests;
#[cfg(test)]
mod person_service_tests;
