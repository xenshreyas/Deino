//! The folder port and its SQLite implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::FolderId;
use sqlx::SqliteConnection;

use super::{Folder, FolderMove};

/// Where folders are stored.
#[async_trait]
pub trait FolderRepo: Send + Sync {
    /// Adds a folder. Fails if a sibling has the same name.
    async fn create_folder(&self, folder: &Folder) -> Result<(), DbError>;

    /// The folder with `id`.
    async fn folder(&self, id: &FolderId) -> Result<Option<Folder>, DbError>;

    /// Every folder, grouped by parent (top level first) and by name within
    /// a parent, so the caller can build the tree in one pass.
    async fn folders(&self) -> Result<Vec<Folder>, DbError>;

    /// The folders directly under `parent` (`None`: the top level), by name.
    async fn children(&self, parent: Option<&FolderId>) -> Result<Vec<Folder>, DbError>;

    /// Saves the folder's name, icon, description and `updated_at`. Moving
    /// is [`FolderRepo::move_folder`], which guards against cycles.
    async fn update_folder(&self, folder: &Folder) -> Result<bool, DbError>;

    /// Moves the folder under `parent` (`None`: the top level), unless that
    /// would put it inside its own subtree. This is the only writer of
    /// `parent_id` after creation, so the tree can never form a cycle.
    async fn move_folder(
        &self,
        id: &FolderId,
        parent: Option<&FolderId>,
        at: DateTime<Utc>,
    ) -> Result<FolderMove, DbError>;

    /// Deletes the folder and its subfolders; their meetings stay.
    async fn delete_folder(&self, id: &FolderId) -> Result<bool, DbError>;
}

/// [`FolderRepo`] and [`MeetingFolderRepo`](super::MeetingFolderRepo) over
/// Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteFolderRepo {
    pub(super) database: Database,
}

impl SqliteFolderRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl FolderRepo for SqliteFolderRepo {
    async fn create_folder(&self, folder: &Folder) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO folders (id, parent_id, name, icon, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            folder.id,
            folder.parent_id,
            folder.name,
            folder.icon,
            folder.description,
            folder.created_at,
            folder.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn folder(&self, id: &FolderId) -> Result<Option<Folder>, DbError> {
        let folder = sqlx::query_file_as!(Folder, "src/folders/queries/folder.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(folder)
    }

    async fn folders(&self) -> Result<Vec<Folder>, DbError> {
        let folders = sqlx::query_file_as!(Folder, "src/folders/queries/folders.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(folders)
    }

    async fn children(&self, parent: Option<&FolderId>) -> Result<Vec<Folder>, DbError> {
        let folders = sqlx::query_file_as!(Folder, "src/folders/queries/children.sql", parent)
            .fetch_all(self.database.reader())
            .await?;
        Ok(folders)
    }

    async fn update_folder(&self, folder: &Folder) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE folders SET name = ?1, icon = ?2, description = ?3, updated_at = ?4 WHERE id = ?5",
            folder.name,
            folder.icon,
            folder.description,
            folder.updated_at,
            folder.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn move_folder(
        &self,
        id: &FolderId,
        parent: Option<&FolderId>,
        at: DateTime<Utc>,
    ) -> Result<FolderMove, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        if let Some(parent) = parent
            && is_within(&mut transaction, parent, id).await?
        {
            return Ok(FolderMove::WouldCycle);
        }
        let result = sqlx::query!(
            "UPDATE folders SET parent_id = ?1, updated_at = ?2 WHERE id = ?3",
            parent,
            at,
            id,
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(if result.rows_affected() > 0 {
            FolderMove::Moved
        } else {
            FolderMove::NotFound
        })
    }

    async fn delete_folder(&self, id: &FolderId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM folders WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// True when `folder` is `ancestor` or sits somewhere below it.
async fn is_within(
    connection: &mut SqliteConnection,
    folder: &FolderId,
    ancestor: &FolderId,
) -> Result<bool, DbError> {
    let within = sqlx::query_scalar!(
        r#"WITH RECURSIVE lineage (id) AS (
            SELECT ?1
            UNION
            SELECT folders.parent_id FROM folders JOIN lineage ON folders.id = lineage.id
            WHERE folders.parent_id IS NOT NULL
        )
        SELECT EXISTS (SELECT 1 FROM lineage WHERE id = ?2) AS "within!: bool""#,
        folder,
        ancestor,
    )
    .fetch_one(connection)
    .await?;
    Ok(within)
}
