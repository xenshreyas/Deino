//! An in-memory store for every meetings port, for service and command tests.

use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{MeetingId, MeetingSource, MeetingStatus, TemplateId};

use super::{
    Meeting, MeetingAudio, MeetingLifecycleRepo, MeetingLink, MeetingRepo, MeetingState,
    MeetingTiming, MeetingTitle, MeetingTrashRepo, NewMeeting, StatusChange,
};
use crate::testing::lock;

/// Keeps meetings in a `Vec` and honours the [`MeetingRepo`],
/// [`MeetingLifecycleRepo`] and [`MeetingTrashRepo`] contracts, including
/// their orderings and the trash hiding meetings from `list`.
#[derive(Debug, Default)]
pub struct FakeMeetingRepo {
    rows: Mutex<Vec<Row>>,
}

#[derive(Clone, Debug)]
struct Row {
    meeting: Meeting,
    state: MeetingState,
}

impl FakeMeetingRepo {
    /// A repository that already holds `meetings`, each `ready` and `live`.
    pub fn with(meetings: impl IntoIterator<Item = Meeting>) -> Self {
        let rows = meetings.into_iter().map(|meeting| Row {
            state: default_state(&meeting.id, MeetingStatus::Ready, MeetingSource::Live),
            meeting,
        });
        Self {
            rows: Mutex::new(rows.collect()),
        }
    }

    fn rows(&self) -> MutexGuard<'_, Vec<Row>> {
        lock(&self.rows)
    }

    /// Runs `change` on the row with `id`, stamping `updated_at`; `false` if missing.
    fn update(&self, id: &MeetingId, at: DateTime<Utc>, change: impl FnOnce(&mut Row)) -> bool {
        let mut rows = self.rows();
        let Some(row) = rows.iter_mut().find(|row| row.meeting.id == *id) else {
            return false;
        };
        change(row);
        row.meeting.updated_at = at;
        true
    }

    /// Meetings whose state passes `keep`, sorted by `key` descending.
    fn listed<K: Ord>(
        &self,
        keep: impl Fn(&MeetingState) -> bool,
        key: impl Fn(&Row) -> K,
    ) -> Vec<Meeting> {
        let mut rows: Vec<Row> = self
            .rows()
            .iter()
            .filter(|row| keep(&row.state))
            .cloned()
            .collect();
        rows.sort_by_key(|row| std::cmp::Reverse(key(row)));
        rows.into_iter().map(|row| row.meeting).collect()
    }
}

fn default_state(id: &MeetingId, status: MeetingStatus, source: MeetingSource) -> MeetingState {
    MeetingState {
        id: id.clone(),
        status,
        source,
        started_at: None,
        ended_at: None,
        duration: None,
        calendar_event_id: None,
        series_id: None,
        template_id: None,
        deleted_at: None,
        audio: MeetingAudio::default(),
    }
}

#[async_trait]
impl MeetingRepo for FakeMeetingRepo {
    async fn list(&self) -> Result<Vec<Meeting>, DbError> {
        let newest_first = |row: &Row| (row.meeting.created_at, row.meeting.id.clone());
        Ok(self.listed(|state| state.deleted_at.is_none(), newest_first))
    }

    async fn find(&self, id: &MeetingId) -> Result<Option<Meeting>, DbError> {
        let rows = self.rows();
        Ok(rows
            .iter()
            .find(|row| row.meeting.id == *id)
            .map(|row| row.meeting.clone()))
    }

    async fn rename(
        &self,
        id: &MeetingId,
        title: &MeetingTitle,
        at: DateTime<Utc>,
    ) -> Result<Option<Meeting>, DbError> {
        self.update(id, at, |row| {
            title.as_str().clone_into(&mut row.meeting.title);
        });
        self.find(id).await
    }
}

#[async_trait]
impl MeetingLifecycleRepo for FakeMeetingRepo {
    async fn create(&self, new: &NewMeeting) -> Result<(), DbError> {
        let mut rows = self.rows();
        if rows.iter().any(|row| row.meeting.id == new.id) {
            return Err(DbError::Query(sqlx::Error::Protocol(
                "UNIQUE constraint failed: meetings.id".into(),
            )));
        }
        let mut state = default_state(&new.id, new.status, new.source);
        state.started_at = new.started_at;
        let meeting = Meeting {
            id: new.id.clone(),
            title: new.title.as_str().to_owned(),
            created_at: new.created_at,
            updated_at: new.created_at,
        };
        rows.push(Row { meeting, state });
        Ok(())
    }

    async fn state(&self, id: &MeetingId) -> Result<Option<MeetingState>, DbError> {
        let rows = self.rows();
        Ok(rows
            .iter()
            .find(|row| row.meeting.id == *id)
            .map(|row| row.state.clone()))
    }

    async fn list_by_status(&self, status: MeetingStatus) -> Result<Vec<Meeting>, DbError> {
        let keep = |state: &MeetingState| state.status == status && state.deleted_at.is_none();
        Ok(self.listed(keep, |row| (row.meeting.created_at, row.meeting.id.clone())))
    }

    async fn set_status(&self, id: &MeetingId, change: StatusChange) -> Result<bool, DbError> {
        let current = self.state(id).await?.map(|state| state.status);
        if current != Some(change.from) {
            return Ok(false);
        }
        Ok(self.update(id, change.at, |row| row.state.status = change.to))
    }

    async fn set_timing(
        &self,
        id: &MeetingId,
        timing: &MeetingTiming,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        Ok(self.update(id, at, |row| {
            row.state.started_at = timing.started_at;
            row.state.ended_at = timing.ended_at;
            row.state.duration = timing.duration;
        }))
    }

    async fn set_audio(
        &self,
        id: &MeetingId,
        audio: &MeetingAudio,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        Ok(self.update(id, at, |row| row.state.audio = audio.clone()))
    }

    async fn link(
        &self,
        id: &MeetingId,
        link: &MeetingLink,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        Ok(self.update(id, at, |row| {
            row.state.calendar_event_id = link.calendar_event_id;
            row.state.series_id.clone_from(&link.series_id);
        }))
    }

    async fn set_template(
        &self,
        id: &MeetingId,
        template: Option<&TemplateId>,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        Ok(self.update(id, at, |row| row.state.template_id = template.cloned()))
    }
}

#[async_trait]
impl MeetingTrashRepo for FakeMeetingRepo {
    async fn trash(&self, id: &MeetingId, at: DateTime<Utc>) -> Result<bool, DbError> {
        let in_trash = self
            .state(id)
            .await?
            .map(|state| state.deleted_at.is_some());
        if in_trash != Some(false) {
            return Ok(false);
        }
        Ok(self.update(id, at, |row| row.state.deleted_at = Some(at)))
    }

    async fn restore(&self, id: &MeetingId, at: DateTime<Utc>) -> Result<bool, DbError> {
        let in_trash = self
            .state(id)
            .await?
            .map(|state| state.deleted_at.is_some());
        if in_trash != Some(true) {
            return Ok(false);
        }
        Ok(self.update(id, at, |row| row.state.deleted_at = None))
    }

    async fn list_trashed(&self) -> Result<Vec<Meeting>, DbError> {
        let trashed_first = |row: &Row| (row.state.deleted_at, row.meeting.id.clone());
        Ok(self.listed(|state| state.deleted_at.is_some(), trashed_first))
    }

    async fn delete(&self, id: &MeetingId) -> Result<bool, DbError> {
        let mut rows = self.rows();
        let before = rows.len();
        rows.retain(|row| row.meeting.id != *id);
        Ok(rows.len() < before)
    }
}
