//! An in-memory [`ActionItemRepo`].

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{ActionItemId, ActionItemStatus, MeetingId, PersonId};

use super::{ActionItem, ActionItemRepo};
use crate::testing::lock;

/// Keeps items in a `Vec` and honours the [`ActionItemRepo`] contract,
/// including `completed_at` following the status and the list orders.
#[derive(Debug, Default)]
pub struct FakeActionItemRepo {
    items: Mutex<Vec<ActionItem>>,
}

impl FakeActionItemRepo {
    /// Items passing `keep`, dated ones soonest first, then undated ones.
    fn by_due_date(&self, keep: impl Fn(&ActionItem) -> bool) -> Vec<ActionItem> {
        let mut items: Vec<ActionItem> = lock(&self.items)
            .iter()
            .filter(|item| keep(item))
            .cloned()
            .collect();
        items.sort_by_key(|item| (item.due_at.is_none(), item.due_at, item.id));
        items
    }
}

#[async_trait]
impl ActionItemRepo for FakeActionItemRepo {
    async fn create(&self, item: &ActionItem) -> Result<(), DbError> {
        lock(&self.items).push(item.clone());
        Ok(())
    }

    async fn find(&self, id: &ActionItemId) -> Result<Option<ActionItem>, DbError> {
        Ok(lock(&self.items)
            .iter()
            .find(|item| item.id == *id)
            .cloned())
    }

    async fn update(&self, item: &ActionItem) -> Result<bool, DbError> {
        let mut items = lock(&self.items);
        let Some(stored) = items.iter_mut().find(|stored| stored.id == item.id) else {
            return Ok(false);
        };
        stored.meeting_id.clone_from(&item.meeting_id);
        stored.title.clone_from(&item.title);
        stored.description.clone_from(&item.description);
        stored.assignee_person_id = item.assignee_person_id;
        stored.due_at = item.due_at;
        stored.priority = item.priority;
        stored.updated_at = item.updated_at;
        Ok(true)
    }

    async fn set_status(
        &self,
        id: &ActionItemId,
        status: ActionItemStatus,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let mut items = lock(&self.items);
        let Some(item) = items.iter_mut().find(|item| item.id == *id) else {
            return Ok(false);
        };
        item.completed_at = match status {
            ActionItemStatus::Done => item.completed_at.or(Some(at)),
            ActionItemStatus::Open => None,
        };
        item.status = status;
        item.updated_at = at;
        Ok(true)
    }

    async fn delete(&self, id: &ActionItemId) -> Result<bool, DbError> {
        let mut items = lock(&self.items);
        let before = items.len();
        items.retain(|item| item.id != *id);
        Ok(items.len() < before)
    }

    async fn list_open(&self) -> Result<Vec<ActionItem>, DbError> {
        Ok(self.by_due_date(|item| item.status == ActionItemStatus::Open))
    }

    async fn list_for_meeting(&self, meeting: &MeetingId) -> Result<Vec<ActionItem>, DbError> {
        let mut items: Vec<ActionItem> = lock(&self.items)
            .iter()
            .filter(|item| item.meeting_id.as_ref() == Some(meeting))
            .cloned()
            .collect();
        items.sort_by_key(|item| (item.created_at, item.id));
        Ok(items)
    }

    async fn list_for_assignee(
        &self,
        person: &PersonId,
        status: ActionItemStatus,
    ) -> Result<Vec<ActionItem>, DbError> {
        Ok(self
            .by_due_date(|item| item.assignee_person_id == Some(*person) && item.status == status))
    }
}
