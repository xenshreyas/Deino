//! Writes over many action items at once, each in one transaction, so a
//! bulk edit or a regeneration is all-or-nothing and the UI hears a single
//! `DbChanged`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{ActionItemId, ActionItemStatus, MeetingId};
use sqlx::SqliteConnection;

use super::{ActionItem, SqliteActionItemRepo};

/// Many-item writes.
#[async_trait]
pub trait ActionItemBatch: Send + Sync {
    /// Replaces the meeting's AI items the user never touched (see
    /// [`ActionItem::is_untouched_ai`]) with `fresh`. Returns how many it removed.
    async fn replace_untouched_ai(
        &self,
        meeting: &MeetingId,
        fresh: &[ActionItem],
    ) -> Result<u64, DbError>;

    /// Saves each item's editable fields, as [`ActionItemRepo::update`](super::ActionItemRepo::update)
    /// does for one. Returns how many existed.
    async fn update_many(&self, items: &[ActionItem]) -> Result<u64, DbError>;

    /// Opens or completes every item in `ids` at `at`, keeping
    /// `completed_at` in step. Returns how many existed.
    async fn set_status_many(
        &self,
        ids: &[ActionItemId],
        status: ActionItemStatus,
        at: DateTime<Utc>,
    ) -> Result<u64, DbError>;

    /// Deletes every item in `ids`. Returns how many existed.
    async fn delete_many(&self, ids: &[ActionItemId]) -> Result<u64, DbError>;
}

#[async_trait]
impl ActionItemBatch for SqliteActionItemRepo {
    async fn replace_untouched_ai(
        &self,
        meeting: &MeetingId,
        fresh: &[ActionItem],
    ) -> Result<u64, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let removed = sqlx::query!(
            "DELETE FROM action_items
             WHERE meeting_id = ?1 AND created_by = 'ai' AND status = 'open'
                 AND updated_at = created_at",
            meeting,
        )
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        for item in fresh {
            insert(&mut transaction, item).await?;
        }
        transaction.commit().await?;
        Ok(removed)
    }

    async fn update_many(&self, items: &[ActionItem]) -> Result<u64, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let mut updated = 0;
        for item in items {
            updated += update(&mut transaction, item).await?;
        }
        transaction.commit().await?;
        Ok(updated)
    }

    async fn set_status_many(
        &self,
        ids: &[ActionItemId],
        status: ActionItemStatus,
        at: DateTime<Utc>,
    ) -> Result<u64, DbError> {
        let done = status == ActionItemStatus::Done;
        let mut transaction = self.database.writer().begin().await?;
        let mut changed = 0;
        for id in ids {
            changed += sqlx::query!(
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
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        }
        transaction.commit().await?;
        Ok(changed)
    }

    async fn delete_many(&self, ids: &[ActionItemId]) -> Result<u64, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let mut deleted = 0;
        for id in ids {
            deleted += sqlx::query!("DELETE FROM action_items WHERE id = ?1", id)
                .execute(&mut *transaction)
                .await?
                .rows_affected();
        }
        transaction.commit().await?;
        Ok(deleted)
    }
}

/// Inserts `item` as it is.
pub(super) async fn insert(
    connection: &mut SqliteConnection,
    item: &ActionItem,
) -> Result<(), DbError> {
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
    .execute(connection)
    .await?;
    Ok(())
}

/// Saves `item`'s editable fields; the number of rows it changed.
pub(super) async fn update(
    connection: &mut SqliteConnection,
    item: &ActionItem,
) -> Result<u64, DbError> {
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
    .execute(connection)
    .await?;
    Ok(result.rows_affected())
}
