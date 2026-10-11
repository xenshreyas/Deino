//! The read side of action items: [`ActionItemReader`] lists them as
//! [`ActionItemView`]s for an [`ItemQuery`], through the index its
//! [`AccessPath`] names.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemPriority, ActionItemStatus, MeetingId, Millis,
    PersonId, SegmentId,
};

use super::{AccessPath, ActionItemView, ItemQuery, SqliteActionItemRepo};

/// Reads action items with what their rows show.
#[async_trait]
pub trait ActionItemReader: Send + Sync {
    /// The items `query` matches: dated ones soonest first, then undated ones.
    async fn list(&self, query: &ItemQuery) -> Result<Vec<ActionItemView>, DbError>;

    /// One item, whatever its meeting's state.
    async fn view(&self, id: &ActionItemId) -> Result<Option<ActionItemView>, DbError>;
}

/// Runs one of the `list_by_*.sql` files, which share their parameters.
macro_rules! list_query {
    ($file:literal, $query:expr, $reader:expr) => {{
        let query: &ItemQuery = $query;
        let [first, second] = &query.statuses;
        sqlx::query_file_as!(
            ActionItemView,
            $file,
            first,
            second,
            query.person,
            query.assigned_only,
            query.not_person,
            query.unassigned_only,
            query.meeting,
            query.tag,
            query.company,
            query.folder,
            query.due_from,
            query.due_before,
            query.undated_only,
            query.held_from,
            query.held_before,
        )
        .fetch_all($reader)
        .await
    }};
}

#[async_trait]
impl ActionItemReader for SqliteActionItemRepo {
    async fn list(&self, query: &ItemQuery) -> Result<Vec<ActionItemView>, DbError> {
        let reader = self.database.reader();
        let items = match query.access_path() {
            AccessPath::Meeting => list_query!(
                "src/action_items/queries/list_by_meeting.sql",
                query,
                reader
            )?,
            AccessPath::Person => {
                list_query!("src/action_items/queries/list_by_person.sql", query, reader)?
            }
            AccessPath::Status => {
                list_query!("src/action_items/queries/list_by_status.sql", query, reader)?
            }
        };
        Ok(items)
    }

    async fn view(&self, id: &ActionItemId) -> Result<Option<ActionItemView>, DbError> {
        let item = sqlx::query_file_as!(ActionItemView, "src/action_items/queries/view.sql", id)
            .fetch_optional(self.database.reader())
            .await?;
        Ok(item)
    }
}

#[cfg(test)]
#[path = "list_repo_tests.rs"]
mod tests;
