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
}
