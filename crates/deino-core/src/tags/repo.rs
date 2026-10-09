//! The tag port and its SQLite implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{PaletteColor, TagId};

use super::Tag;

/// Where tags are stored.
#[async_trait]
pub trait TagRepo: Send + Sync {
    /// Adds a tag. Fails if another tag has the same name, ignoring case.
    async fn create_tag(&self, tag: &Tag) -> Result<(), DbError>;

    /// The tag with `id`.
    async fn tag(&self, id: &TagId) -> Result<Option<Tag>, DbError>;

    /// The tag named `name`, ignoring case.
    async fn tag_by_name(&self, name: &str) -> Result<Option<Tag>, DbError>;

    /// Every tag, by name ignoring case.
    async fn tags(&self) -> Result<Vec<Tag>, DbError>;

    /// Saves every editable field of `tag` and its `updated_at`.
    async fn update_tag(&self, tag: &Tag) -> Result<bool, DbError>;

    /// Deletes the tag and takes it off every meeting.
    async fn delete_tag(&self, id: &TagId) -> Result<bool, DbError>;
}

/// [`TagRepo`] and [`MeetingTagRepo`](super::MeetingTagRepo) over Deino's
/// SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteTagRepo {
    pub(super) database: Database,
}

impl SqliteTagRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl TagRepo for SqliteTagRepo {
    async fn create_tag(&self, tag: &Tag) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO tags (id, name, color, description, auto_apply, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            tag.id,
            tag.name,
            tag.color,
            tag.description,
            tag.auto_apply,
            tag.created_at,
            tag.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn tag(&self, id: &TagId) -> Result<Option<Tag>, DbError> {
        let tag = sqlx::query_file_as!(Tag, "src/tags/queries/tag.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(tag)
    }

    async fn tag_by_name(&self, name: &str) -> Result<Option<Tag>, DbError> {
        let tag = sqlx::query_file_as!(Tag, "src/tags/queries/tag_by_name.sql", name)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(tag)
    }

    async fn tags(&self) -> Result<Vec<Tag>, DbError> {
        let tags = sqlx::query_file_as!(Tag, "src/tags/queries/tags.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(tags)
    }

    async fn update_tag(&self, tag: &Tag) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE tags SET name = ?1, color = ?2, description = ?3, auto_apply = ?4, updated_at = ?5
             WHERE id = ?6",
            tag.name,
            tag.color,
            tag.description,
            tag.auto_apply,
            tag.updated_at,
            tag.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_tag(&self, id: &TagId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM tags WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
