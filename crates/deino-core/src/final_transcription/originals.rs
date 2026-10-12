//! The transcript a cloud transcription replaced (CAP-14), kept in
//! `original_transcripts` so the user can bring it back.
//!
//! Only the first replaced transcript is kept: a second cloud run replaces
//! the cloud transcript, and the one made on this Mac is what the user may
//! want back. Speakers are not kept; restoring carries them over from the
//! transcript in place ([`carried_speaker`](super::carried_speaker)).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{Channel, MeetingId, Millis};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::segments::{Segment, WordTiming};

/// One kept line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptLine {
    /// The track it was heard on.
    pub channel: Channel,
    /// When it starts.
    pub start: Millis,
    /// When it ends.
    pub end: Millis,
    /// What was said, with the user's edits.
    pub text: String,
    /// Its words, when they were timed.
    pub words: Vec<WordTiming>,
}

impl KeptLine {
    /// The line of `segment` (its speaker is not kept).
    pub fn of(segment: &Segment) -> Self {
        Self {
            channel: segment.channel,
            start: segment.start,
            end: segment.end,
            text: segment.text.clone(),
            words: segment.words.clone(),
        }
    }
}

/// Where replaced transcripts are kept.
#[async_trait]
pub trait OriginalTranscripts: Send + Sync {
    /// Keeps `lines` as `meeting`'s original transcript, unless one is
    /// already kept.
    async fn keep(
        &self,
        meeting: &MeetingId,
        lines: &[KeptLine],
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// The kept transcript of `meeting`, if any.
    async fn kept(&self, meeting: &MeetingId) -> Result<Option<Vec<KeptLine>>, DbError>;

    /// Forgets the kept transcript of `meeting`; succeeds when there is none.
    async fn forget(&self, meeting: &MeetingId) -> Result<(), DbError>;
}

/// [`OriginalTranscripts`] over SQLite.
#[derive(Clone)]
pub struct SqliteOriginalTranscripts {
    database: Database,
}

impl SqliteOriginalTranscripts {
    /// The repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl OriginalTranscripts for SqliteOriginalTranscripts {
    async fn keep(
        &self,
        meeting: &MeetingId,
        lines: &[KeptLine],
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let lines = Json(lines);
        sqlx::query!(
            "INSERT INTO original_transcripts (meeting_id, lines, replaced_by, replaced_at)
             VALUES (?1, ?2, 'elevenlabs', ?3)
             ON CONFLICT (meeting_id) DO NOTHING",
            meeting,
            lines,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn kept(&self, meeting: &MeetingId) -> Result<Option<Vec<KeptLine>>, DbError> {
        let lines = sqlx::query_scalar!(
            r#"SELECT lines AS "lines: Json<Vec<KeptLine>>"
               FROM original_transcripts WHERE meeting_id = ?1"#,
            meeting,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(lines.map(|Json(lines)| lines))
    }

    async fn forget(&self, meeting: &MeetingId) -> Result<(), DbError> {
        sqlx::query!(
            "DELETE FROM original_transcripts WHERE meeting_id = ?1",
            meeting
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeOriginalTranscripts;

#[cfg(any(test, feature = "test-support"))]
mod fake {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use deino_db::DbError;
    use deino_domain::MeetingId;

    use super::{KeptLine, OriginalTranscripts};
    use crate::testing::lock;

    /// Kept transcripts in memory.
    #[derive(Debug, Default)]
    pub struct FakeOriginalTranscripts {
        kept: Mutex<HashMap<MeetingId, Vec<KeptLine>>>,
    }

    #[async_trait]
    impl OriginalTranscripts for FakeOriginalTranscripts {
        async fn keep(
            &self,
            meeting: &MeetingId,
            lines: &[KeptLine],
            _at: DateTime<Utc>,
        ) -> Result<(), DbError> {
            lock(&self.kept)
                .entry(meeting.clone())
                .or_insert_with(|| lines.to_vec());
            Ok(())
        }

        async fn kept(&self, meeting: &MeetingId) -> Result<Option<Vec<KeptLine>>, DbError> {
            Ok(lock(&self.kept).get(meeting).cloned())
        }

        async fn forget(&self, meeting: &MeetingId) -> Result<(), DbError> {
            lock(&self.kept).remove(meeting);
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "originals_tests.rs"]
mod tests;
