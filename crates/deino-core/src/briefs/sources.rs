//! [`BriefSources`]: gathers a brief's [`BriefInputs`] from the user's own
//! data only (§8 D12, D13: no web, no email): the attendees as people and
//! their companies with their summaries, earlier meetings with them, the
//! last meeting of the series, their open action items and the invitation.
//! With redaction on (NOT-09), every text is redacted before the prompt is
//! built, so secrets never reach the model or the fingerprint.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use deino_db::Database;
use deino_domain::{ActionItemStatus, CompanyId, Email, LocalZone, MeetingId, PersonId};

use super::collect::{event_header, event_source, item_source, meeting_source, summary_sources};
use super::{BriefError, BriefInputs, Source};
use crate::action_items::{ActionItem, ActionItemRepo, SqliteActionItemRepo};
use crate::calendar::CalendarEvent;
use crate::calendar::linking::{RelatedMeeting, RelatedService, RelationKey, SqliteRelatedRepo};
use crate::companies::{Company, CompanyRepo, SqliteCompanyRepo};
use crate::entities::{EntityDigest, EntityReads, SqliteEntityRepo};
use crate::people::{Person, PersonRepo, SqlitePeopleRepo};
use crate::redaction::{NoRedaction, TextRedactor};

/// Earlier meetings read per person or company.
const DIGESTS_PER_ENTITY: u32 = 4;
/// Earlier meetings a brief is written from, most recent first.
const MAX_MEETINGS: usize = 4;
/// Open action items a brief is written from.
const MAX_ITEMS: usize = 6;

/// The repositories a brief is gathered from.
#[derive(Clone)]
pub struct BriefSources {
    /// People, by address.
    pub people: Arc<dyn PersonRepo>,
    /// Companies.
    pub companies: Arc<dyn CompanyRepo>,
    /// Earlier meetings with a person or company.
    pub reads: Arc<dyn EntityReads>,
    /// Open action items.
    pub items: Arc<dyn ActionItemRepo>,
    /// The series' last meeting (ORG-03).
    pub related: Arc<RelatedService>,
    /// The user's time zone, for dates in the prompt.
    pub zone: Arc<dyn LocalZone>,
    /// Hides secrets in every text (NOT-09).
    pub redactor: Arc<dyn TextRedactor>,
}

impl BriefSources {
    /// Every source over Deino's SQLite database, without redaction
    /// ([`Self::with_redactor`]).
    pub fn sqlite(database: &Database, zone: Arc<dyn LocalZone>) -> Self {
        let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
        let related = RelatedService::new(
            Arc::new(SqliteRelatedRepo::new(database.clone())),
            people.clone(),
        );
        Self {
            people,
            companies: Arc::new(SqliteCompanyRepo::new(database.clone())),
            reads: Arc::new(SqliteEntityRepo::new(database.clone())),
            items: Arc::new(SqliteActionItemRepo::new(database.clone())),
            related: Arc::new(related),
            zone,
            redactor: Arc::new(NoRedaction),
        }
    }

    /// These sources, with `redactor` hiding secrets in every text.
    #[must_use]
    pub fn with_redactor(self, redactor: Arc<dyn TextRedactor>) -> Self {
        Self { redactor, ..self }
    }

    /// The inputs of `event`'s brief; `own_notes` are the event's own
    /// meeting notes, never a source.
    pub async fn gather(
        &self,
        event: &CalendarEvent,
        own_notes: &[MeetingId],
    ) -> Result<BriefInputs, BriefError> {
        let people = self.attendees(event).await?;
        let companies = self.companies_of(&people).await?;
        let mut sources = self
            .meetings(event, (&people, &companies), own_notes)
            .await?;
        sources.extend(self.open_items(&people).await?);
        sources.extend(summary_sources(&people, &companies));
        sources.extend(event_source(event));
        let header = event_header(event, &people, &companies, self.zone.as_ref());
        Ok(self.redacted(header, sources).await)
    }

    /// The attendees Deino knows as people, the user and merged duplicates
    /// left out (a merged person is read as whom they were merged into).
    async fn attendees(&self, event: &CalendarEvent) -> Result<Vec<Person>, BriefError> {
        let mut people: BTreeMap<PersonId, Person> = BTreeMap::new();
        let emails = event.attendees.iter().filter_map(|a| a.email.as_deref());
        for email in emails.filter_map(|email| Email::parse(email).ok()) {
            let Some(mut person) = self.people.find_by_email(&email).await? else {
                continue;
            };
            if let Some(survivor) = person.merged_into {
                let Some(found) = self.people.find(&survivor).await? else {
                    continue;
                };
                person = found;
            }
            if !person.is_me {
                people.insert(person.id, person);
            }
        }
        Ok(people.into_values().collect())
    }

    /// The companies `people` work at.
    async fn companies_of(&self, people: &[Person]) -> Result<Vec<Company>, BriefError> {
        let ids: BTreeSet<CompanyId> = people.iter().filter_map(|p| p.company_id).collect();
        let mut companies = Vec::with_capacity(ids.len());
        for id in ids {
            companies.extend(self.companies.find(&id).await?);
        }
        Ok(companies)
    }

    /// Earlier meetings with the people or their companies, most recent
    /// first, with the series' last meeting marked (or added, when the
    /// people changed since).
    async fn meetings(
        &self,
        event: &CalendarEvent,
        (people, companies): (&[Person], &[Company]),
        own_notes: &[MeetingId],
    ) -> Result<Vec<Source>, BriefError> {
        let mut earlier = self.earlier(event, people, companies).await?;
        earlier.retain(|digest| !own_notes.contains(&digest.meeting_id));
        earlier.truncate(MAX_MEETINGS);
        let series = self.series_previous(event, people, own_notes).await?;
        if let Some(previous) = &series
            && !earlier.iter().any(|d| d.meeting_id == previous.id)
        {
            earlier.push(EntityDigest {
                meeting_id: previous.id.clone(),
                title: previous.title.clone(),
                at: previous.happened_at,
                summary: None,
                notes: None,
            });
        }
        let series_id = series.map(|previous| previous.id);
        let zone = self.zone.as_ref();
        Ok(earlier
            .iter()
            .map(|d| meeting_source(d, series_id.as_ref() == Some(&d.meeting_id), zone))
            .collect())
    }

    /// The digests of meetings with `people` or `companies` before the
    /// event, most recent first.
    async fn earlier(
        &self,
        event: &CalendarEvent,
        people: &[Person],
        companies: &[Company],
    ) -> Result<Vec<EntityDigest>, BriefError> {
        let mut digests: BTreeMap<MeetingId, EntityDigest> = BTreeMap::new();
        for person in people {
            let found = self
                .reads
                .person_digests(&person.id, DIGESTS_PER_ENTITY)
                .await?;
            digests.extend(found.into_iter().map(|d| (d.meeting_id.clone(), d)));
        }
        for company in companies {
            let found = self
                .reads
                .company_digests(&company.id, DIGESTS_PER_ENTITY)
                .await?;
            digests.extend(found.into_iter().map(|d| (d.meeting_id.clone(), d)));
        }
        let mut earlier: Vec<EntityDigest> = digests
            .into_values()
            .filter(|digest| digest.at < event.start_at)
            .collect();
        earlier.sort_by_key(|digest| std::cmp::Reverse(digest.at));
        Ok(earlier)
    }

    /// The meeting "Previous in series" would show for the event (ORG-03).
    async fn series_previous(
        &self,
        event: &CalendarEvent,
        people: &[Person],
        own_notes: &[MeetingId],
    ) -> Result<Option<RelatedMeeting>, BriefError> {
        let key = RelationKey {
            // An ID no meeting has when the event has no note yet.
            meeting: own_notes
                .first()
                .cloned()
                .unwrap_or_else(|| MeetingId::new(event.id.to_string())),
            title: event.title.clone(),
            series_id: event.series_id.clone(),
            happened_at: event.start_at,
            attendees: people.iter().map(|person| person.id).collect(),
        };
        let related = self.related.related_to(&key).await?;
        Ok(related.previous)
    }

    /// The people's open action items, oldest commitments first.
    async fn open_items(&self, people: &[Person]) -> Result<Vec<Source>, BriefError> {
        let mut items: BTreeMap<_, ActionItem> = BTreeMap::new();
        for person in people {
            let open = self
                .items
                .list_for_assignee(&person.id, ActionItemStatus::Open)
                .await?;
            items.extend(open.into_iter().map(|item| (item.id, item)));
        }
        let mut items: Vec<ActionItem> = items.into_values().collect();
        items.sort_by_key(|item| item.created_at);
        let names: BTreeMap<PersonId, &str> =
            people.iter().map(|p| (p.id, p.name.as_str())).collect();
        Ok(items
            .iter()
            .take(MAX_ITEMS)
            .map(|item| item_source(item, &names))
            .collect())
    }

    /// `header` and `sources` with every text redacted, in one call.
    async fn redacted(&self, header: String, sources: Vec<Source>) -> BriefInputs {
        let mut texts = Vec::with_capacity(1 + sources.len() * 2);
        texts.push(header);
        for source in &sources {
            texts.push(source.heading.clone());
            texts.push(source.body.clone());
        }
        let mut texts = self.redactor.redact(texts).await.into_iter();
        let event = texts.next().unwrap_or_default();
        let sources = sources
            .into_iter()
            .map(|source| Source {
                heading: texts.next().unwrap_or(source.heading),
                body: texts.next().unwrap_or(source.body),
                citation: source.citation,
            })
            .collect();
        BriefInputs { event, sources }
    }
}
