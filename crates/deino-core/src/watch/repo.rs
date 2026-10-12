//! The watched folders' storage port and its SQLite adapter:
//! `watched_folders` and the `watched_files` ledger.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{ImportFailure, WatchedFileId, WatchedFolderId};

use super::{HandledFile, Sighting, WatchFailure, WatchedFolder};

/// Where watched folders and the files they handled are stored.
#[async_trait]
pub trait WatchRepo: Send + Sync {
    /// Every watched folder, oldest first.
    async fn folders(&self) -> Result<Vec<WatchedFolder>, DbError>;

    /// Adds a folder; fails with a unique violation when its path is watched.
    async fn add_folder(&self, folder: &WatchedFolder) -> Result<(), DbError>;

    /// Removes a folder and its ledger; false when it does not exist.
    async fn remove_folder(&self, id: &WatchedFolderId) -> Result<bool, DbError>;

    /// Turns "Move to processed" on or off; false when the folder is gone.
    async fn set_move_to_processed(&self, id: &WatchedFolderId, on: bool) -> Result<bool, DbError>;

    /// Whether the folder already handled this exact file.
    async fn handled(&self, folder: &WatchedFolderId, file: &Sighting) -> Result<bool, DbError>;

    /// Enters a handled file in the ledger; a file already there is kept.
    async fn record(&self, file: &HandledFile) -> Result<(), DbError>;

    /// Failed files not dismissed, newest first.
    async fn failures(&self) -> Result<Vec<WatchFailure>, DbError>;

    /// Deletes a failed file's row so the next scan imports it again;
    /// returns its folder, or `None` when no failure has the ID.
    async fn forget_failure(&self, id: &WatchedFileId) -> Result<Option<WatchedFolderId>, DbError>;

    /// Hides a failure for good; false when no listed failure has the ID.
    async fn dismiss_failure(&self, id: &WatchedFileId, at: DateTime<Utc>)
    -> Result<bool, DbError>;
}

/// [`WatchRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteWatchRepo {
    database: Database,
}

impl SqliteWatchRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl WatchRepo for SqliteWatchRepo {
    async fn folders(&self) -> Result<Vec<WatchedFolder>, DbError> {
        let folders = sqlx::query_as!(
            WatchedFolder,
            r#"SELECT id AS "id: WatchedFolderId", path,
                      move_to_processed AS "move_to_processed: bool",
                      created_at AS "created_at: DateTime<Utc>"
               FROM watched_folders
               ORDER BY created_at, path"#
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(folders)
    }

    async fn add_folder(&self, folder: &WatchedFolder) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO watched_folders (id, path, move_to_processed, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            folder.id,
            folder.path,
            folder.move_to_processed,
            folder.created_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn remove_folder(&self, id: &WatchedFolderId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM watched_folders WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_move_to_processed(&self, id: &WatchedFolderId, on: bool) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE watched_folders SET move_to_processed = ?1 WHERE id = ?2",
            on,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn handled(&self, folder: &WatchedFolderId, file: &Sighting) -> Result<bool, DbError> {
        let path = file.path.to_string_lossy();
        let size = size_of(file);
        let handled = sqlx::query_scalar!(
            r#"SELECT EXISTS (
                   SELECT 1 FROM watched_files
                   WHERE folder_id = ?1 AND path = ?2 AND size_bytes = ?3 AND modified_ms = ?4
               ) AS "handled!: bool""#,
            folder,
            path,
            size,
            file.modified_ms,
        )
        .fetch_one(self.database.reader())
        .await?;
        Ok(handled)
    }

    async fn record(&self, file: &HandledFile) -> Result<(), DbError> {
        let path = file.sighting.path.to_string_lossy();
        let size = size_of(&file.sighting);
        sqlx::query!(
            "INSERT INTO watched_files
                 (id, folder_id, path, size_bytes, modified_ms, meeting_id, failure, handled_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (folder_id, path, size_bytes, modified_ms) DO NOTHING",
            file.id,
            file.folder_id,
            path,
            size,
            file.sighting.modified_ms,
            file.meeting_id,
            file.failure,
            file.handled_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn failures(&self) -> Result<Vec<WatchFailure>, DbError> {
        let failures = sqlx::query_as!(
            WatchFailure,
            r#"SELECT id AS "id: WatchedFileId", folder_id AS "folder_id: WatchedFolderId", path,
                      failure AS "failure!: ImportFailure",
                      handled_at AS "failed_at: DateTime<Utc>"
               FROM watched_files
               WHERE failure IS NOT NULL AND dismissed_at IS NULL
               ORDER BY handled_at DESC"#
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(failures)
    }

    async fn forget_failure(&self, id: &WatchedFileId) -> Result<Option<WatchedFolderId>, DbError> {
        let folder = sqlx::query_scalar!(
            r#"DELETE FROM watched_files WHERE id = ?1 AND failure IS NOT NULL
               RETURNING folder_id AS "folder_id: WatchedFolderId""#,
            id,
        )
        .fetch_optional(self.database.writer())
        .await?;
        Ok(folder)
    }

    async fn dismiss_failure(
        &self,
        id: &WatchedFileId,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE watched_files SET dismissed_at = ?1
             WHERE id = ?2 AND failure IS NOT NULL AND dismissed_at IS NULL",
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// A file's size as SQLite stores it (files never reach `i64::MAX` bytes).
fn size_of(file: &Sighting) -> i64 {
    i64::try_from(file.size_bytes).unwrap_or(i64::MAX)
}

#[cfg(test)]
#[path = "repo_tests.rs"]
mod tests;
