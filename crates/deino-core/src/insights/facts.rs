//! What conditions read about a meeting, and which past meetings an insight
//! can run on.
//!
//! [`MeetingRecords`] reads the stored facts; whether a meeting is external
//! depends on the user's own domains (Settings → Profile), which
//! [`OwnDomains`] supplies, so the repository never reads settings.

use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{
    Audience, CalendarId, Email, FolderId, MeetingFacts, MeetingId, PersonId, TagId, TemplateId,
};
use serde::Serialize;

use super::SqliteInsightRepo;
use crate::profile::InternalDomains;
use crate::settings::SettingsService;

/// A meeting's stored facts, before its audience is judged.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeetingRecord {
    /// Every fact but `has_external`.
    pub facts: MeetingFacts,
    /// The attendees' addresses.
    pub attendee_emails: Vec<Email>,
}

impl MeetingRecord {
    /// The facts, with the audience judged against `domains`.
    pub fn facts(self, domains: &InternalDomains) -> MeetingFacts {
        let audience = domains.classify(&self.attendee_emails);
        MeetingFacts {
            has_external: audience == Some(Audience::External),
            ..self.facts
        }
    }
}

/// A past meeting an insight can run on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PastMeeting {
    /// The meeting.
    pub id: MeetingId,
    /// Its title.
    pub title: String,
    /// When it started (or was saved).
    pub started_at: DateTime<Utc>,
}

/// Reads what conditions match on.
#[async_trait]
pub trait MeetingRecords: Send + Sync {
    /// The meeting's facts; `None` when no meeting outside the trash has the ID.
    async fn record(&self, meeting: &MeetingId) -> Result<Option<MeetingRecord>, DbError>;

    /// Finished meetings with a transcript, newest first.
    async fn past_meetings(&self) -> Result<Vec<PastMeeting>, DbError>;
}

/// The user's own email domains.
#[async_trait]
pub trait OwnDomains: Send + Sync {
    /// Who counts as internal.
    async fn internal_domains(&self) -> InternalDomains;
}

#[async_trait]
impl OwnDomains for SettingsService {
    async fn internal_domains(&self) -> InternalDomains {
        InternalDomains::of(&self.get().await.profile)
    }
}

#[async_trait]
impl MeetingRecords for SqliteInsightRepo {
    async fn record(&self, meeting: &MeetingId) -> Result<Option<MeetingRecord>, DbError> {
        let reader = self.database.reader();
        let Some(row) = sqlx::query_file!("src/insights/queries/meeting_facts.sql", meeting)
            .fetch_optional(reader)
            .await?
        else {
            return Ok(None);
        };
        let tag_ids = sqlx::query_scalar!(
            r#"SELECT tag_id AS "tag_id: TagId" FROM meeting_tags WHERE meeting_id = ?1"#,
            meeting,
        )
        .fetch_all(reader)
        .await?;
        let folder_ids = sqlx::query_scalar!(
            r#"SELECT folder_id AS "folder_id: FolderId" FROM meeting_folders WHERE meeting_id = ?1"#,
            meeting,
        )
        .fetch_all(reader)
        .await?;
        let attendee_emails = self.attendee_emails(meeting).await?;
        Ok(Some(MeetingRecord {
            facts: MeetingFacts {
                title: row.title,
                template_id: row.template_id,
                tag_ids: tag_ids.into_iter().collect(),
                folder_ids: folder_ids.into_iter().collect(),
                participant_ids: self.participants(meeting).await?,
                invitee_emails: attendee_emails.iter().cloned().collect(),
                attendee_domains: domains(&attendee_emails),
                has_external: false,
                duration_s: row
                    .duration_s
                    .and_then(|seconds| u32::try_from(seconds).ok()),
                calendar_id: row.calendar_id,
            },
            attendee_emails,
        }))
    }

    async fn past_meetings(&self) -> Result<Vec<PastMeeting>, DbError> {
        let meetings = sqlx::query_file_as!(PastMeeting, "src/insights/queries/past_meetings.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(meetings)
    }
}

impl SqliteInsightRepo {
    /// Everyone who took part: attendees of any role and named speakers.
    async fn participants(&self, meeting: &MeetingId) -> Result<BTreeSet<PersonId>, DbError> {
        let people = sqlx::query_scalar!(
            r#"SELECT person_id AS "person_id!: PersonId" FROM meeting_attendees
               WHERE meeting_id = ?1
               UNION
               SELECT person_id AS "person_id!: PersonId" FROM speakers
               WHERE meeting_id = ?1 AND person_id IS NOT NULL"#,
            meeting,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(people.into_iter().collect())
    }

    async fn attendee_emails(&self, meeting: &MeetingId) -> Result<Vec<Email>, DbError> {
        let emails = sqlx::query_scalar!(
            r#"SELECT p.email AS "email!: Email" FROM meeting_attendees AS a
               JOIN people AS p ON p.id = a.person_id
               WHERE a.meeting_id = ?1 AND p.email IS NOT NULL"#,
            meeting,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(emails)
    }
}

/// The distinct domains of `emails`.
fn domains(emails: &[Email]) -> BTreeSet<String> {
    emails
        .iter()
        .map(|email| email.domain().to_owned())
        .collect()
}
