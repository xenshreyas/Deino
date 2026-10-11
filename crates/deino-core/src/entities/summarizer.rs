//! [`EntitySummarizer`]: refreshes a person's or company's AI summary on
//! the gateway's `entity.summary` route (§4.3: the fast model).
//!
//! The briefing is stored in `summary_md` with `summary_updated_at`, by a
//! single-field write, so notes the user is typing are never overwritten.
//! Reasoning envelopes a local model writes inline and an outer fence are
//! removed from the answer ([`visible_answer`]); an answer that is only
//! reasoning (or nothing) is not stored.

use std::sync::Arc;

use deino_domain::{ActionItemStatus, CompanyId, PersonId};
use deino_llm::{Gateway, Route};

use super::person_service::entity_item;
use super::summary_prompt::{
    SUMMARY_MEETINGS, Subject, SubjectKind, summary_prompt, visible_answer,
};
use super::{EntityActionItem, EntityError, EntityPorts};
use crate::companies::Company;
use crate::people::Person;

/// People named in a company's briefing details, at most.
const MAX_NAMED_PEOPLE: usize = 12;

/// What a refresh did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SummaryOutcome {
    /// A new summary was stored.
    Refreshed,
    /// Nothing has been written about them yet; the summary is unchanged.
    NoContent,
    /// The model answered with nothing usable; the summary is unchanged.
    EmptyAnswer,
    /// No LLM could be wired at startup; the summary is unchanged.
    NoModel,
}

impl SummaryOutcome {
    /// True when a new summary was stored.
    pub fn refreshed(self) -> bool {
        self == Self::Refreshed
    }
}

/// Whose summary is being stored.
#[derive(Clone, Copy)]
enum Target {
    Person(PersonId),
    Company(CompanyId),
}

/// Writes entity summaries with the LLM gateway.
pub struct EntitySummarizer {
    ports: EntityPorts,
    gateway: Option<Arc<Gateway>>,
}

impl EntitySummarizer {
    /// A summariser over the entity ports and the gateway; without a
    /// gateway every refresh is [`SummaryOutcome::NoModel`].
    pub fn new(ports: EntityPorts, gateway: Option<Arc<Gateway>>) -> Self {
        Self { ports, gateway }
    }

    /// Refreshes the summary of `id` (or of whom they were merged into).
    pub async fn summarize_person(&self, id: &PersonId) -> Result<SummaryOutcome, EntityError> {
        let person = self.ports.people.find(id).await?;
        let person = person.ok_or(EntityError::PersonNotFound(*id))?;
        let person = self.ports.directory.surviving_person(person).await?;
        let digests = self
            .ports
            .reads
            .person_digests(&person.id, SUMMARY_MEETINGS)
            .await?;
        let items = self.open_items(&[person.id]).await?;
        let subject = self.person_subject(&person).await?;
        match self.answer(&subject, &digests, &items).await? {
            Ok(answer) => self.store(Target::Person(person.id), &answer).await,
            Err(skipped) => Ok(skipped),
        }
    }

    /// Refreshes the summary of `id` (or of the company it was merged into).
    pub async fn summarize_company(&self, id: &CompanyId) -> Result<SummaryOutcome, EntityError> {
        let company = self.ports.companies.find(id).await?;
        let company = company.ok_or(EntityError::CompanyNotFound(*id))?;
        let company = self.ports.directory.surviving_company(company).await?;
        let digests = self
            .ports
            .reads
            .company_digests(&company.id, SUMMARY_MEETINGS)
            .await?;
        let people = self.ports.reads.person_rows(Some(&company.id)).await?;
        let ids: Vec<PersonId> = people.iter().map(|person| person.id).collect();
        let items = self.open_items(&ids).await?;
        let names: Vec<&str> = people
            .iter()
            .take(MAX_NAMED_PEOPLE)
            .map(|p| p.name.as_str())
            .collect();
        let subject = company_subject(&company, &names);
        match self.answer(&subject, &digests, &items).await? {
            Ok(answer) => self.store(Target::Company(company.id), &answer).await,
            Err(skipped) => Ok(skipped),
        }
    }

    /// The model's raw answer, or why there is none.
    async fn answer(
        &self,
        subject: &Subject,
        digests: &[super::EntityDigest],
        items: &[EntityActionItem],
    ) -> Result<Result<String, SummaryOutcome>, EntityError> {
        let Some(gateway) = &self.gateway else {
            return Ok(Err(SummaryOutcome::NoModel));
        };
        let Some(prompt) = summary_prompt(subject, digests, items) else {
            return Ok(Err(SummaryOutcome::NoContent));
        };
        let completion = gateway.complete(Route::EntitySummary, prompt).await?;
        Ok(Ok(completion.text))
    }

    /// Cleans `answer` and saves it as the summary of `target`, stamped now.
    async fn store(&self, target: Target, answer: &str) -> Result<SummaryOutcome, EntityError> {
        let Some(summary) = visible_answer(answer) else {
            return Ok(SummaryOutcome::EmptyAnswer);
        };
        let now = self.ports.clock.now();
        let writes = &self.ports.writes;
        match target {
            Target::Person(id) => writes.set_person_summary(&id, &summary, now).await?,
            Target::Company(id) => writes.set_company_summary(&id, &summary, now).await?,
        };
        Ok(SummaryOutcome::Refreshed)
    }

    /// The open action items assigned to any of `people`.
    async fn open_items(&self, people: &[PersonId]) -> Result<Vec<EntityActionItem>, EntityError> {
        let mut items = Vec::new();
        for person in people {
            let open = self
                .ports
                .action_items
                .list_for_assignee(person, ActionItemStatus::Open)
                .await?;
            items.extend(open.into_iter().map(entity_item));
        }
        Ok(items)
    }

    /// The person's details: title, company and address domain.
    async fn person_subject(&self, person: &Person) -> Result<Subject, EntityError> {
        let mut details = Vec::new();
        if let Some(title) = &person.title {
            details.push(format!("Title: {title}"));
        }
        if let Some(company) = person.company_id
            && let Some(company) = self.ports.companies.find(&company).await?
        {
            details.push(format!("Company: {}", company.name));
        }
        Ok(Subject {
            kind: SubjectKind::Person,
            name: person.name.clone(),
            details,
            notes: person.notes_md.clone(),
        })
    }
}

/// The company's details: its domain and the people the user meets there.
fn company_subject(company: &Company, people: &[&str]) -> Subject {
    let mut details = Vec::new();
    if let Some(domain) = &company.domain {
        details.push(format!("Website: {}", domain.as_str()));
    }
    if !people.is_empty() {
        details.push(format!(
            "People the user meets there: {}",
            people.join(", ")
        ));
    }
    Subject {
        kind: SubjectKind::Company,
        name: company.name.clone(),
        details,
        notes: company.notes_md.clone(),
    }
}

#[cfg(test)]
mod summarizer_tests;
