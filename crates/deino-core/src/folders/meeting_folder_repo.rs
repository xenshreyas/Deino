//! The meeting-in-folder port: which meetings sit in which folders.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{FolderId, MeetingId};

use super::{Folder, SqliteFolderRepo};
use crate::meetings::Meeting;

/// Files meetings into folders. A meeting can sit in many folders.
#[async_trait]
pub trait MeetingFolderRepo: Send + Sync {
    /// Puts the meeting in the folder at `at`; filing it again changes nothing.
    async fn file_meeting(
        &self,
        meeting: &MeetingId,
        folder: &FolderId,
        at: DateTime<Utc>,
    ) -> Result<(), DbError>;

    /// Takes the meeting out of the folder.
    async fn unfile_meeting(&self, meeting: &MeetingId, folder: &FolderId)
    -> Result<bool, DbError>;

    /// The meetings outside the trash in the folder, newest first.
    async fn meetings_in(&self, folder: &FolderId) -> Result<Vec<Meeting>, DbError>;

    /// The folders the meeting sits in, by name.
    async fn folders_of(&self, meeting: &MeetingId) -> Result<Vec<Folder>, DbError>;

    /// How many meetings outside the trash each folder holds directly;
    /// folders holding none are left out.
    async fn meeting_counts(&self) -> Result<Vec<(FolderId, u32)>, DbError>;

    /// The meetings outside the trash that sit in no folder, newest first.
    async fn unfiled(&self) -> Result<Vec<Meeting>, DbError>;

    /// How many meetings [`MeetingFolderRepo::unfiled`] would list.
    async fn unfiled_count(&self) -> Result<u32, DbError>;
}

#[async_trait]
impl MeetingFolderRepo for SqliteFolderRepo {
    async fn file_meeting(
        &self,
        meeting: &MeetingId,
        folder: &FolderId,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO meeting_folders (meeting_id, folder_id, added_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (meeting_id, folder_id) DO NOTHING",
            meeting,
            folder,
            at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn unfile_meeting(
        &self,
        meeting: &MeetingId,
        folder: &FolderId,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "DELETE FROM meeting_folders WHERE meeting_id = ?1 AND folder_id = ?2",
            meeting,
            folder,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn meetings_in(&self, folder: &FolderId) -> Result<Vec<Meeting>, DbError> {
        let meetings = sqlx::query_file_as!(Meeting, "src/folders/queries/meetings_in.sql", folder)
            .fetch_all(self.database.reader())
            .await?;
        Ok(meetings)
    }

    async fn folders_of(&self, meeting: &MeetingId) -> Result<Vec<Folder>, DbError> {
        let folders = sqlx::query_file_as!(Folder, "src/folders/queries/folders_of.sql", meeting)
            .fetch_all(self.database.reader())
            .await?;
        Ok(folders)
    }

    async fn meeting_counts(&self) -> Result<Vec<(FolderId, u32)>, DbError> {
        let rows = sqlx::query_file!("src/folders/queries/meeting_counts.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| (row.folder_id, saturating_u32(row.meetings)))
            .collect())
    }

    async fn unfiled(&self) -> Result<Vec<Meeting>, DbError> {
        let meetings = sqlx::query_file_as!(Meeting, "src/folders/queries/unfiled.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(meetings)
    }

    async fn unfiled_count(&self) -> Result<u32, DbError> {
        let count = sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!: i64" FROM meetings AS m
               WHERE m.deleted_at IS NULL
                 AND NOT EXISTS (SELECT 1 FROM meeting_folders AS mf WHERE mf.meeting_id = m.id)"#
        )
        .fetch_one(self.database.reader())
        .await?;
        Ok(saturating_u32(count))
    }
}

/// A SQLite count as a `u32`; counts are never negative, and no one has
/// four billion meetings.
fn saturating_u32(count: i64) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}
