//! The meeting-facts port: what the rule engine reads about a meeting.
//!
//! Defined here, by its one consumer, so the folders feature depends on
//! exactly the three facts its rules match and not on the meetings, tags
//! and people repositories as a whole (interface segregation).

use std::collections::BTreeSet;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{Email, MeetingId, SeriesId, TagId};

use super::{MeetingFacts, SqliteFolderRepo};

/// Reads the facts folder rules match on.
#[async_trait]
pub trait MeetingFactsReader: Send + Sync {
    /// The meeting's series, tags and attendee domains; `None` when no
    /// meeting outside the trash has that ID.
    async fn facts(&self, meeting: &MeetingId) -> Result<Option<MeetingFacts>, DbError>;
}

#[async_trait]
impl MeetingFactsReader for SqliteFolderRepo {
    async fn facts(&self, meeting: &MeetingId) -> Result<Option<MeetingFacts>, DbError> {
        let reader = self.database.reader();
        let Some(series_id) = sqlx::query_scalar!(
            r#"SELECT series_id AS "series_id: SeriesId" FROM meetings
               WHERE id = ?1 AND deleted_at IS NULL"#,
            meeting,
        )
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
        let emails = sqlx::query_scalar!(
            r#"SELECT p.email AS "email!: Email" FROM meeting_attendees AS a
               JOIN people AS p ON p.id = a.person_id
               WHERE a.meeting_id = ?1 AND p.email IS NOT NULL"#,
            meeting,
        )
        .fetch_all(reader)
        .await?;
        Ok(Some(MeetingFacts {
            series_id,
            tag_ids: tag_ids.into_iter().collect(),
            attendee_domains: domains(&emails),
        }))
    }
}

/// The distinct domains of `emails`.
fn domains(emails: &[Email]) -> BTreeSet<String> {
    emails
        .iter()
        .map(|email| email.domain().to_owned())
        .collect()
}
