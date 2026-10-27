//! The `update_entities` stage's work (order 90, ORG-04): after a meeting
//! is processed, its people and companies catch up with it.
//!
//! 1. **Named speakers join the meeting.** A transcript speaker the user (or
//!    CAP-04's voice match) named is a person; one not invited is added as
//!    `speaker_only`, so the meeting shows on their page. The
//!    `link_speakers` stage (order 47) does this before the notes are
//!    written, so they see every attendee and are not out of date at once;
//!    `update_entities` does it again for speakers named since.
//! 2. **Summaries refresh.** Every attendee except the user, and every
//!    company they work at, gets a new briefing on `entity.summary`, at
//!    most [`MAX_SUMMARIES`] in all, people first.
//! 3. **Logos.** Each of those companies' favicon is fetched if it never
//!    was, when the user allows it (a refused fetch is skipped quietly).

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{AttendeeRole, CompanyId, MeetingId, PersonId};

use super::{EntityError, EntityPorts, EntitySummarizer, LogoService};
use crate::people::Attendee;
use crate::segments::SpeakerRepo;

/// Briefings refreshed after one meeting, at most: a 30-person all-hands
/// should not cost 30 model calls.
pub const MAX_SUMMARIES: usize = 12;

/// What updating a meeting's entities did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityUpdate {
    /// Named speakers newly added as attendees.
    pub speakers_linked: usize,
    /// Summaries refreshed.
    pub summaries: usize,
}

/// Brings a processed meeting's people and companies up to date.
#[async_trait]
pub trait EntityUpdater: Send + Sync {
    /// Updates `meeting`'s entities (see the module docs). Dropping the
    /// future cancels the model call in flight.
    async fn update_entities(&self, meeting: &MeetingId) -> Result<EntityUpdate, EntityError>;

    /// Adds every named speaker who is not an attendee yet (step 1 alone);
    /// returns how many joined. An updater without speakers links none.
    async fn link_speakers(&self, _meeting: &MeetingId) -> Result<usize, EntityError> {
        Ok(0)
    }
}

/// What [`EntityUpdateService`] is built from.
#[derive(Clone)]
pub struct EntityUpdatePorts {
    /// People, companies and attendance.
    pub entities: EntityPorts,
    /// The meeting's transcript speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// The summariser (it skips every refresh when no LLM could be wired).
    pub summarizer: Arc<EntitySummarizer>,
    /// Company logos; `None` when the app has no cache folder.
    pub logos: Option<Arc<LogoService>>,
}

/// [`EntityUpdater`] over the entity services.
pub struct EntityUpdateService {
    ports: EntityUpdatePorts,
}

impl EntityUpdateService {
    /// Creates the service over its ports.
    pub fn new(ports: EntityUpdatePorts) -> Self {
        Self { ports }
    }

    /// The meeting's people other than the user, and their companies.
    async fn participants(
        &self,
        meeting: &MeetingId,
    ) -> Result<(Vec<PersonId>, Vec<CompanyId>), EntityError> {
        let entities = &self.ports.entities;
        let mut people = Vec::new();
        let mut companies = BTreeSet::new();
        for attendee in entities.attendees.attendees(meeting).await? {
            let Some(person) = entities.people.find(&attendee.person_id).await? else {
                continue;
            };
            let person = entities.directory.surviving_person(person).await?;
            if person.is_me || people.contains(&person.id) {
                continue;
            }
            companies.extend(person.company_id);
            people.push(person.id);
        }
        Ok((people, companies.into_iter().collect()))
    }

    /// Refreshes up to [`MAX_SUMMARIES`] briefings, people first; returns
    /// how many were stored.
    async fn summarize(
        &self,
        people: &[PersonId],
        companies: &[CompanyId],
    ) -> Result<usize, EntityError> {
        let summarizer = &self.ports.summarizer;
        let mut asked = 0;
        let mut refreshed = 0;
        for person in people.iter().take(MAX_SUMMARIES) {
            asked += 1;
            refreshed += usize::from(summarizer.summarize_person(person).await?.refreshed());
        }
        for company in companies.iter().take(MAX_SUMMARIES.saturating_sub(asked)) {
            refreshed += usize::from(summarizer.summarize_company(company).await?.refreshed());
        }
        Ok(refreshed)
    }

    /// Fetches each company's logo once, when the user allows it.
    async fn fetch_logos(&self, companies: &[CompanyId]) {
        let Some(logos) = &self.ports.logos else {
            return;
        };
        for company in companies {
            match logos.ensure(company, false).await {
                Ok(_) => {}
                Err(EntityError::Logo(error)) if error.is_refused() => return,
                Err(error) => {
                    log::warn!(target: "entities", "no logo for company {company}: {error}");
                }
            }
        }
    }
}

#[async_trait]
impl EntityUpdater for EntityUpdateService {
    async fn update_entities(&self, meeting: &MeetingId) -> Result<EntityUpdate, EntityError> {
        let speakers_linked = self.link_speakers(meeting).await?;
        let (people, companies) = self.participants(meeting).await?;
        let summaries = self.summarize(&people, &companies).await?;
        self.fetch_logos(&companies).await;
        log::info!(
            target: "entities",
            "meeting {meeting}: {speakers_linked} speakers linked, {summaries} summaries refreshed"
        );
        Ok(EntityUpdate {
            speakers_linked,
            summaries,
        })
    }

    async fn link_speakers(&self, meeting: &MeetingId) -> Result<usize, EntityError> {
        let attendees = &self.ports.entities.attendees;
        let present: BTreeSet<PersonId> = attendees
            .attendees(meeting)
            .await?
            .into_iter()
            .map(|attendee| attendee.person_id)
            .collect();
        let named: BTreeSet<PersonId> = self
            .ports
            .speakers
            .speakers(meeting)
            .await?
            .into_iter()
            .filter_map(|speaker| speaker.person_id)
            .filter(|person| !present.contains(person))
            .collect();
        for person in &named {
            let attendee = Attendee {
                meeting_id: meeting.clone(),
                person_id: *person,
                role: AttendeeRole::SpeakerOnly,
                rsvp: None,
            };
            attendees.upsert_attendee(&attendee).await?;
        }
        Ok(named.len())
    }
}

#[cfg(test)]
mod updater_tests;
