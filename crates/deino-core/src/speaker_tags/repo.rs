//! The speaker-tag ports and their SQLite implementation.
//!
//! [`SpeakerTagWriter`] is what the watcher appends through while a meeting
//! records; [`SpeakerTagReader`] is what `identify_speakers` reads.
//! [`SqliteSpeakerTagRepo`] implements both over `speaker_tag_spans`.

use async_trait::async_trait;
use deino_db::{Database, DbError};
use deino_domain::{CallApp, DisplayName, MeetingId, Millis, TagSpan};

/// Appends a recording's speaker tags.
#[async_trait]
pub trait SpeakerTagWriter: Send + Sync {
    /// Stores `spans` for `meeting`, in one transaction.
    async fn append(&self, meeting: &MeetingId, spans: &[TagSpan]) -> Result<(), DbError>;
}

/// Reads a meeting's speaker tags.
#[async_trait]
pub trait SpeakerTagReader: Send + Sync {
    /// Every span of `meeting`, in time order.
    async fn spans(&self, meeting: &MeetingId) -> Result<Vec<TagSpan>, DbError>;
}

/// [`SpeakerTagWriter`] and [`SpeakerTagReader`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteSpeakerTagRepo {
    database: Database,
}

impl SqliteSpeakerTagRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl SpeakerTagWriter for SqliteSpeakerTagRepo {
    async fn append(&self, meeting: &MeetingId, spans: &[TagSpan]) -> Result<(), DbError> {
        if spans.is_empty() {
            return Ok(());
        }
        let mut transaction = self.database.writer().begin().await?;
        for span in spans {
            sqlx::query!(
                "INSERT INTO speaker_tag_spans (meeting_id, start_ms, end_ms, display_name, app)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                meeting,
                span.start,
                span.end,
                span.name,
                span.app,
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl SpeakerTagReader for SqliteSpeakerTagRepo {
    async fn spans(&self, meeting: &MeetingId) -> Result<Vec<TagSpan>, DbError> {
        let rows = sqlx::query!(
            r#"SELECT start_ms AS "start: Millis", end_ms AS "end: Millis",
                      display_name AS "name: DisplayName", app AS "app: CallApp"
               FROM speaker_tag_spans WHERE meeting_id = ?1 ORDER BY start_ms, id"#,
            meeting
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| TagSpan {
                start: row.start,
                end: row.end,
                name: row.name,
                app: row.app,
            })
            .collect())
    }
}

#[cfg(test)]
#[path = "repo_tests.rs"]
mod tests;
