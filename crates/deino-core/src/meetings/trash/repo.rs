//! [`TrashBin`]: the trash port for many meetings at once, and its SQLite
//! implementation. Every write is one statement (or one transaction) over
//! a JSON array of IDs, so trashing 2,000 meetings is one commit and one
//! `DbChanged`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::MeetingId;
use sqlx::types::Json;

use super::{StoredFiles, TrashedMeeting};
use crate::meetings::SqliteMeetingRepo;

/// Moves meetings into and out of the trash, and deletes them for good.
#[async_trait]
pub trait TrashBin: Send + Sync {
    /// Moves `ids` to the trash at `at`; returns those that were not there.
    async fn trash_all(
        &self,
        ids: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, DbError>;

    /// Takes `ids` out of the trash; returns those that were in it.
    async fn restore_all(
        &self,
        ids: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, DbError>;

    /// The trash, most recently trashed first.
    async fn trashed(&self) -> Result<Vec<TrashedMeeting>, DbError>;

    /// Meetings trashed before `cutoff`, oldest first.
    async fn expired(&self, cutoff: DateTime<Utc>) -> Result<Vec<MeetingId>, DbError>;

    /// The recording files of `ids` and of the parts merged into them.
    async fn stored_files(&self, ids: &[MeetingId]) -> Result<Vec<StoredFiles>, DbError>;

    /// Deletes those of `ids` that are in the trash, with every row that
    /// belongs to them; returns the deleted ones.
    async fn delete_trashed(&self, ids: &[MeetingId]) -> Result<Vec<MeetingId>, DbError>;
}

/// One `trashed.sql` row.
struct TrashedRow {
    id: MeetingId,
    title: String,
    created_at: DateTime<Utc>,
    deleted_at: DateTime<Utc>,
}

/// One `stored_files.sql` row.
struct FilesRow {
    meeting: MeetingId,
    owner: MeetingId,
    audio_path: Option<String>,
    audio_mic_path: Option<String>,
    audio_system_path: Option<String>,
    video_path: Option<String>,
}

#[async_trait]
impl TrashBin for SqliteMeetingRepo {
    async fn trash_all(
        &self,
        ids: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, DbError> {
        let ids = Json(ids);
        let changed = sqlx::query_scalar!(
            r#"UPDATE meetings SET deleted_at = ?1, updated_at = ?1
            WHERE id IN (SELECT value FROM json_each(?2)) AND deleted_at IS NULL
            RETURNING id AS "id!: MeetingId""#,
            at,
            ids,
        )
        .fetch_all(self.database.writer())
        .await?;
        Ok(changed)
    }

    async fn restore_all(
        &self,
        ids: &[MeetingId],
        at: DateTime<Utc>,
    ) -> Result<Vec<MeetingId>, DbError> {
        let ids = Json(ids);
        let changed = sqlx::query_scalar!(
            r#"UPDATE meetings SET deleted_at = NULL, updated_at = ?1
            WHERE id IN (SELECT value FROM json_each(?2)) AND deleted_at IS NOT NULL
            RETURNING id AS "id!: MeetingId""#,
            at,
            ids,
        )
        .fetch_all(self.database.writer())
        .await?;
        Ok(changed)
    }

    async fn trashed(&self) -> Result<Vec<TrashedMeeting>, DbError> {
        let rows = sqlx::query_file_as!(TrashedRow, "src/meetings/trash/queries/trashed.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| TrashedMeeting::new(row.id, row.title, row.created_at, row.deleted_at))
            .collect())
    }

    async fn expired(&self, cutoff: DateTime<Utc>) -> Result<Vec<MeetingId>, DbError> {
        let ids = sqlx::query_file_scalar!("src/meetings/trash/queries/expired.sql", cutoff)
            .fetch_all(self.database.reader())
            .await?;
        Ok(ids)
    }

    async fn stored_files(&self, ids: &[MeetingId]) -> Result<Vec<StoredFiles>, DbError> {
        let ids = Json(ids);
        let rows =
            sqlx::query_file_as!(FilesRow, "src/meetings/trash/queries/stored_files.sql", ids)
                .fetch_all(self.database.reader())
                .await?;
        Ok(rows.into_iter().map(FilesRow::into_files).collect())
    }

    async fn delete_trashed(&self, ids: &[MeetingId]) -> Result<Vec<MeetingId>, DbError> {
        // One statement: foreign keys cascade to every row of each meeting
        // (and its merge parts) in the same implicit transaction.
        let ids = Json(ids);
        let deleted = sqlx::query_scalar!(
            r#"DELETE FROM meetings
            WHERE id IN (SELECT value FROM json_each(?1)) AND deleted_at IS NOT NULL
            RETURNING id AS "id!: MeetingId""#,
            ids,
        )
        .fetch_all(self.database.writer())
        .await?;
        Ok(deleted)
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
