//! [`FakeScreenRepo`]: screen capture's rows in memory, for tests.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::MeetingId;

use super::{NewSnapshot, ScreenRepo, Snapshot};

/// Snapshots and choices kept in memory.
#[derive(Debug, Default)]
pub struct FakeScreenRepo {
    snapshots: Mutex<Vec<NewSnapshot>>,
    choices: Mutex<HashMap<MeetingId, bool>>,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[async_trait]
impl ScreenRepo for FakeScreenRepo {
    async fn add_snapshot(
        &self,
        snapshot: &NewSnapshot,
        _at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let mut snapshots = locked(&self.snapshots);
        let known = snapshots
            .iter()
            .any(|kept| kept.meeting == snapshot.meeting && kept.file_name == snapshot.file_name);
        if !known {
            snapshots.push(snapshot.clone());
        }
        Ok(())
    }

    async fn snapshots(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError> {
        let mut rows: Vec<Snapshot> = locked(&self.snapshots)
            .iter()
            .zip(1_i64..)
            .filter(|(kept, _)| &kept.meeting == meeting)
            .map(|(kept, id)| Snapshot {
                id,
                at: kept.at,
                file_name: kept.file_name.clone(),
                width: kept.width,
                height: kept.height,
            })
            .collect();
        rows.sort_by_key(|row| (row.at, row.id));
        Ok(rows)
    }

    async fn set_capture(
        &self,
        meeting: &MeetingId,
        capture: bool,
        _at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        locked(&self.choices).insert(meeting.clone(), capture);
        Ok(())
    }

    async fn capture_choice(&self, meeting: &MeetingId) -> Result<Option<bool>, DbError> {
        Ok(locked(&self.choices).get(meeting).copied())
    }
}
