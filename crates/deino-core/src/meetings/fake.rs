//! An in-memory [`MeetingRepo`] for service and command tests.

use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::MeetingId;

use super::{Meeting, MeetingRepo, MeetingTitle};

/// Keeps meetings in a `Vec` and honours the [`MeetingRepo`] contract,
/// including the newest-first order of `list`.
#[derive(Debug, Default)]
pub struct FakeMeetingRepo {
    meetings: Mutex<Vec<Meeting>>,
}

impl FakeMeetingRepo {
    /// A repository that already holds `meetings`.
    pub fn with(meetings: impl IntoIterator<Item = Meeting>) -> Self {
        Self {
            meetings: Mutex::new(meetings.into_iter().collect()),
        }
    }

    fn meetings(&self) -> MutexGuard<'_, Vec<Meeting>> {
        self.meetings.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[async_trait]
impl MeetingRepo for FakeMeetingRepo {
    async fn list(&self) -> Result<Vec<Meeting>, DbError> {
        let mut meetings = self.meetings().clone();
        meetings.sort_by(|a, b| (&b.created_at, &b.id).cmp(&(&a.created_at, &a.id)));
        Ok(meetings)
    }

    async fn find(&self, id: &MeetingId) -> Result<Option<Meeting>, DbError> {
        Ok(self
            .meetings()
            .iter()
            .find(|meeting| meeting.id == *id)
            .cloned())
    }

    async fn rename(
        &self,
        id: &MeetingId,
        title: &MeetingTitle,
        at: DateTime<Utc>,
    ) -> Result<Option<Meeting>, DbError> {
        let mut meetings = self.meetings();
        let Some(meeting) = meetings.iter_mut().find(|meeting| meeting.id == *id) else {
            return Ok(None);
        };
        title.as_str().clone_into(&mut meeting.title);
        meeting.updated_at = at;
        Ok(Some(meeting.clone()))
    }
}
