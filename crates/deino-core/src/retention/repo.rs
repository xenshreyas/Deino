//! [`RetentionStore`]: what retention reads and deletes in the database, and
//! its SQLite implementation. Each deletion is one transaction over a JSON
//! array of IDs, so a sweep is a few commits and a few `DbChanged`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::MeetingId;
use sqlx::types::Json;

use super::MeetingRetention;
use crate::meetings::trash::StoredFiles;

/// The rows retention reads and deletes.
#[async_trait]
pub trait RetentionStore: Send + Sync {
    /// Finished meetings outside the trash saved at or before `cutoff`.
    async fn meetings_due(&self, cutoff: DateTime<Utc>) -> Result<Vec<MeetingId>, DbError>;

    /// Those of them whose transcript is still kept.
    async fn transcripts_due(&self, cutoff: DateTime<Utc>) -> Result<Vec<MeetingId>, DbError>;

    /// The recording files of those of them whose recording is still kept,
    /// with their merge parts' files and their clips.
    async fn recordings_due(&self, cutoff: DateTime<Utc>) -> Result<Vec<StoredFiles>, DbError>;

    /// Deletes the transcripts of `ids` (lines, their search documents and
    /// transcript embeddings) and stamps them deleted at `at`; returns the
    /// meetings stamped.
    async fn delete_transcripts(
        &self,
        ids: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, DbError>;

    /// Forgets the recordings of `ids` (paths, merge parts' paths, clips)
    /// and stamps them deleted at `at`; returns the meetings stamped.
    async fn forget_recordings(
        &self,
        ids: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, DbError>;

    /// What retention removed from `meeting`; `None` when it does not exist.
    async fn status(&self, meeting: &MeetingId) -> Result<Option<MeetingRetention>, DbError>;
}

/// [`RetentionStore`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteRetentionStore {
    database: Database,
}

impl SqliteRetentionStore {
    /// The store over `database`.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// One `recordings_due.sql` row.
struct FilesRow {
    meeting: MeetingId,
    owner: MeetingId,
    audio_path: Option<String>,
    audio_mic_path: Option<String>,
    audio_system_path: Option<String>,
    video_path: Option<String>,
}

#[async_trait]
impl RetentionStore for SqliteRetentionStore {
    async fn meetings_due(&self, cutoff: DateTime<Utc>) -> Result<Vec<MeetingId>, DbError> {
        let ids = sqlx::query_file_scalar!("src/retention/queries/meetings_due.sql", cutoff)
            .fetch_all(self.database.reader())
            .await?;
        Ok(ids)
    }

    async fn transcripts_due(&self, cutoff: DateTime<Utc>) -> Result<Vec<MeetingId>, DbError> {
        let ids = sqlx::query_file_scalar!("src/retention/queries/transcripts_due.sql", cutoff)
            .fetch_all(self.database.reader())
            .await?;
        Ok(ids)
    }

    async fn recordings_due(&self, cutoff: DateTime<Utc>) -> Result<Vec<StoredFiles>, DbError> {
        let rows =
            sqlx::query_file_as!(FilesRow, "src/retention/queries/recordings_due.sql", cutoff)
                .fetch_all(self.database.reader())
                .await?;
        Ok(rows.into_iter().map(FilesRow::into_files).collect())
    }

    async fn delete_transcripts(
        &self,
        ids: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, DbError> {
        // The search triggers drop each line's document (ADR 0032); the
        // transcript's embeddings hold copies of its text, so they go too.
        let ids = Json(ids);
        let mut transaction = self.database.writer().begin().await?;
        sqlx::query!(
            "DELETE FROM transcript_segments WHERE meeting_id IN (SELECT value FROM json_each(?1))",
            ids,
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query!(
            "DELETE FROM embeddings
            WHERE ref_kind = 'transcript' AND meeting_id IN (SELECT value FROM json_each(?1))",
            ids,
        )
        .execute(&mut *transaction)
        .await?;
        let stamped = sqlx::query_scalar!(
            r#"UPDATE meetings SET transcript_deleted_at = ?1, updated_at = ?1
            WHERE id IN (SELECT value FROM json_each(?2)) AND transcript_deleted_at IS NULL
            RETURNING id AS "id!: MeetingId""#,
            at,
            ids,
        )
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(stamped)
    }

    async fn forget_recordings(
        &self,
        ids: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, DbError> {
        let ids = Json(ids);
        let mut transaction = self.database.writer().begin().await?;
        sqlx::query!(
            "DELETE FROM clips WHERE meeting_id IN (SELECT value FROM json_each(?1))",
            ids,
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query!(
            "UPDATE meeting_merge_parts
            SET audio_path = NULL, audio_mic_path = NULL, audio_system_path = NULL, video_path = NULL
            WHERE meeting_id IN (SELECT value FROM json_each(?1))",
            ids,
        )
        .execute(&mut *transaction)
        .await?;
        let stamped = sqlx::query_scalar!(
            r#"UPDATE meetings
            SET audio_path = NULL, audio_mic_path = NULL, audio_system_path = NULL,
                video_path = NULL, audio_deleted_at = ?1, updated_at = ?1
            WHERE id IN (SELECT value FROM json_each(?2)) AND audio_deleted_at IS NULL
            RETURNING id AS "id!: MeetingId""#,
            at,
            ids,
        )
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(stamped)
    }

    async fn status(&self, meeting: &MeetingId) -> Result<Option<MeetingRetention>, DbError> {
        let status = sqlx::query_as!(
            MeetingRetention,
            r#"SELECT
                audio_deleted_at AS "audio_deleted_at: DateTime<Utc>",
                transcript_deleted_at AS "transcript_deleted_at: DateTime<Utc>"
            FROM meetings
            WHERE id = ?1"#,
            meeting,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(status)
    }
}

impl FilesRow {
    fn into_files(self) -> StoredFiles {
        let paths = [
            self.audio_path,
            self.audio_mic_path,
            self.audio_system_path,
            self.video_path,
        ];
        StoredFiles {
            meeting: self.meeting,
            owner: self.owner,
            paths: paths.into_iter().flatten().collect(),
        }
    }
}
