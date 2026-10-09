//! The action item port and its SQLite implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemPriority, ActionItemStatus, MeetingId, PersonId,
    SegmentId,
};

use super::ActionItem;

/// Where action items are stored.
#[async_trait]
pub trait ActionItemRepo: Send + Sync {
    /// Adds an item.
    async fn create(&self, item: &ActionItem) -> Result<(), DbError>;

    /// The item with `id`.
    async fn find(&self, id: &ActionItemId) -> Result<Option<ActionItem>, DbError>;

    /// Saves the item's meeting, title, description, assignee, due date,
    /// priority and `updated_at`. Completion goes through
    /// [`ActionItemRepo::set_status`], which keeps `completed_at` in step.
    async fn update(&self, item: &ActionItem) -> Result<bool, DbError>;

    /// Opens or completes the item at `at`. Completing sets `completed_at`
    /// (keeping the first completion time if it was already done); reopening
    /// clears it.
    async fn set_status(
        &self,
        id: &ActionItemId,
        status: ActionItemStatus,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Deletes the item.
    async fn delete(&self, id: &ActionItemId) -> Result<bool, DbError>;

    /// Every open item: dated ones soonest first, then undated ones.
    async fn list_open(&self) -> Result<Vec<ActionItem>, DbError>;

    /// The meeting's items, in the order they were created.
    async fn list_for_meeting(&self, meeting: &MeetingId) -> Result<Vec<ActionItem>, DbError>;

    /// The person's items in `status`, ordered like [`ActionItemRepo::list_open`].
    async fn list_for_assignee(
        &self,
        person: &PersonId,
        status: ActionItemStatus,
    ) -> Result<Vec<ActionItem>, DbError>;
}

/// [`ActionItemRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteActionItemRepo {
    pub(super) database: Database,
}

impl SqliteActionItemRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl ActionItemRepo for SqliteActionItemRepo {
    async fn create(&self, item: &ActionItem) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO action_items (id, meeting_id, title, description, assignee_person_id, due_at,
                status, priority, source_segment_id, completed_at, created_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            item.id,
            item.meeting_id,
            item.title,
            item.description,
            item.assignee_person_id,
            item.due_at,
            item.status,
            item.priority,
            item.source_segment_id,
            item.completed_at,
            item.created_by,
            item.created_at,
            item.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn find(&self, id: &ActionItemId) -> Result<Option<ActionItem>, DbError> {
        let item = sqlx::query_file_as!(ActionItem, "src/action_items/queries/item.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(item)
    }

    async fn update(&self, item: &ActionItem) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE action_items
             SET meeting_id = ?1, title = ?2, description = ?3, assignee_person_id = ?4, due_at = ?5,
                 priority = ?6, updated_at = ?7
             WHERE id = ?8",
            item.meeting_id,
            item.title,
            item.description,
            item.assignee_person_id,
            item.due_at,
            item.priority,
            item.updated_at,
            item.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_status(
        &self,
        id: &ActionItemId,
        status: ActionItemStatus,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let done = status == ActionItemStatus::Done;
        let result = sqlx::query!(
            "UPDATE action_items
             SET status = ?1,
                 completed_at = CASE WHEN ?2 THEN coalesce(completed_at, ?3) END,
                 updated_at = ?3
             WHERE id = ?4",
            status,
            done,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: &ActionItemId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM action_items WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_open(&self) -> Result<Vec<ActionItem>, DbError> {
        let items = sqlx::query_file_as!(ActionItem, "src/action_items/queries/open.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(items)
    }

    async fn list_for_meeting(&self, meeting: &MeetingId) -> Result<Vec<ActionItem>, DbError> {
        let items = sqlx::query_file_as!(
            ActionItem,
            "src/action_items/queries/for_meeting.sql",
            meeting
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(items)
    }

    async fn list_for_assignee(
        &self,
        person: &PersonId,
        status: ActionItemStatus,
    ) -> Result<Vec<ActionItem>, DbError> {
        let items = sqlx::query_file_as!(
            ActionItem,
            "src/action_items/queries/for_assignee.sql",
            person,
            status,
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(items)
    }
}
