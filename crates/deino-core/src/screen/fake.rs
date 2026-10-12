//! [`FakeScreenRepo`]: screen capture's rows in memory, for tests.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::MeetingId;

use super::{NewSnapshot, ScreenRepo, ScreenText, ScreenTexts, Snapshot, SnapshotTextRepo};

/// One stored snapshot; its id is its place in the list, from 1.
#[derive(Debug)]
struct Row {
    snapshot: NewSnapshot,
    text: Option<String>,
    description: Option<String>,
    deleted: bool,
}

/// Snapshots and choices kept in memory.
#[derive(Debug, Default)]
pub struct FakeScreenRepo {
    rows: Mutex<Vec<Row>>,
    choices: Mutex<HashMap<MeetingId, bool>>,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FakeScreenRepo {
    /// The live rows of `meeting` that pass `keep`, in time order.
    fn select(&self, meeting: &MeetingId, keep: impl Fn(&Row) -> bool) -> Vec<Snapshot> {
        let mut rows: Vec<Snapshot> = locked(&self.rows)
            .iter()
            .zip(1_i64..)
            .filter(|(row, _)| !row.deleted && &row.snapshot.meeting == meeting && keep(row))
            .map(|(row, id)| Snapshot {
                id,
                at: row.snapshot.at,
                file_name: row.snapshot.file_name.clone(),
                width: row.snapshot.width,
                height: row.snapshot.height,
                text: row.text.clone(),
            })
            .collect();
        rows.sort_by_key(|row| (row.at, row.id));
        rows
    }

    /// Changes row `id`, when it exists.
    fn change(&self, id: i64, change: impl FnOnce(&mut Row)) {
        let index = usize::try_from(id - 1).unwrap_or(usize::MAX);
        if let Some(row) = locked(&self.rows).get_mut(index) {
            change(row);
        }
    }

    /// The description stored for snapshot `id`.
    pub fn description(&self, id: i64) -> Option<String> {
        let index = usize::try_from(id - 1).ok()?;
        locked(&self.rows).get(index)?.description.clone()
    }
}

#[async_trait]
impl ScreenRepo for FakeScreenRepo {
    async fn add_snapshot(
        &self,
        snapshot: &NewSnapshot,
        _at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let mut rows = locked(&self.rows);
        let known = rows.iter().any(|row| {
            row.snapshot.meeting == snapshot.meeting && row.snapshot.file_name == snapshot.file_name
        });
        if !known {
            rows.push(Row {
                snapshot: snapshot.clone(),
                text: None,
                description: None,
                deleted: false,
            });
        }
        Ok(())
    }

    async fn snapshots(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError> {
        Ok(self.select(meeting, |_| true))
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

    async fn delete_snapshot(
        &self,
        meeting: &MeetingId,
        id: i64,
    ) -> Result<Option<String>, DbError> {
        let mut name = None;
        self.change(id, |row| {
            if !row.deleted && &row.snapshot.meeting == meeting {
                row.deleted = true;
                name = Some(row.snapshot.file_name.clone());
            }
        });
        Ok(name)
    }
}

#[async_trait]
impl SnapshotTextRepo for FakeScreenRepo {
    async fn unread(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError> {
        Ok(self.select(meeting, |row| row.text.is_none()))
    }

    async fn undescribed(&self, meeting: &MeetingId) -> Result<Vec<Snapshot>, DbError> {
        Ok(self.select(meeting, |row| {
            row.text.is_some() && row.description.is_none()
        }))
    }

    async fn set_text(&self, id: i64, text: &str) -> Result<(), DbError> {
        self.change(id, |row| row.text = Some(text.to_owned()));
        Ok(())
    }

    async fn set_description(&self, id: i64, description: &str) -> Result<(), DbError> {
        self.change(id, |row| row.description = Some(description.to_owned()));
        Ok(())
    }
}

#[async_trait]
impl ScreenTexts for FakeScreenRepo {
    async fn screen_texts(&self, meeting: &MeetingId) -> Result<Vec<ScreenText>, DbError> {
        let rows = locked(&self.rows);
        let mut texts: Vec<ScreenText> = rows
            .iter()
            .filter(|row| !row.deleted && &row.snapshot.meeting == meeting)
            .filter_map(|row| {
                let text = row.text.clone()?;
                (!text.trim().is_empty() || row.description.is_some()).then(|| ScreenText {
                    at: row.snapshot.at,
                    text,
                    description: row.description.clone(),
                })
            })
            .collect();
        texts.sort_by_key(|text| text.at);
        Ok(texts)
    }
}
