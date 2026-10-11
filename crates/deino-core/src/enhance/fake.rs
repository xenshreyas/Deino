//! In-memory enhanced-notes ports, for tests (feature `test-support`).

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{MeetingId, SeriesId};

use super::material::PreviousNotes;
use super::{
    EnhancedNotes, EnhancedNotesReader, EnhancedNotesWriter, NewVersion, VersionEdit, VersionInfo,
};
use crate::testing::lock;

/// [`EnhancedNotesReader`] and [`EnhancedNotesWriter`] over a `Vec`, with
/// the same numbering and newest-only edits as the SQLite repository.
/// Template names are not known here, so versions list none.
#[derive(Debug, Default)]
pub struct FakeEnhancedNotesRepo {
    versions: Mutex<Vec<EnhancedNotes>>,
    previous: Mutex<Option<PreviousNotes>>,
}

impl FakeEnhancedNotesRepo {
    /// Answers every `previous_in_series` with `previous`.
    #[must_use]
    pub fn with_previous(self, previous: PreviousNotes) -> Self {
        *lock(&self.previous) = Some(previous);
        self
    }

    /// Every stored version, in insertion order.
    pub fn all(&self) -> Vec<EnhancedNotes> {
        lock(&self.versions).clone()
    }
}

#[async_trait]
impl EnhancedNotesReader for FakeEnhancedNotesRepo {
    async fn latest(&self, meeting: &MeetingId) -> Result<Option<EnhancedNotes>, DbError> {
        Ok(lock(&self.versions)
            .iter()
            .filter(|notes| notes.meeting_id == *meeting)
            .max_by_key(|notes| notes.version)
            .cloned())
    }

    async fn version(
        &self,
        meeting: &MeetingId,
        version: u32,
    ) -> Result<Option<EnhancedNotes>, DbError> {
        Ok(lock(&self.versions)
            .iter()
            .find(|notes| notes.meeting_id == *meeting && notes.version == version)
            .cloned())
    }

    async fn versions(&self, meeting: &MeetingId) -> Result<Vec<VersionInfo>, DbError> {
        let mut versions: Vec<VersionInfo> = lock(&self.versions)
            .iter()
            .filter(|notes| notes.meeting_id == *meeting)
            .map(|notes| VersionInfo {
                version: notes.version,
                created_at: notes.created_at,
                updated_at: notes.updated_at,
                template_id: notes.template_id.clone(),
                template_name: None,
                model: notes.model.clone(),
                is_user_edited: notes.is_user_edited,
            })
            .collect();
        versions.sort_by_key(|info| std::cmp::Reverse(info.version));
        Ok(versions)
    }

    async fn previous_in_series(
        &self,
        _series: &SeriesId,
        _meeting: &MeetingId,
        _before: DateTime<Utc>,
    ) -> Result<Option<PreviousNotes>, DbError> {
        Ok(lock(&self.previous).clone())
    }
}

#[async_trait]
impl EnhancedNotesWriter for FakeEnhancedNotesRepo {
    async fn insert(&self, version: &NewVersion) -> Result<u32, DbError> {
        let mut versions = lock(&self.versions);
        let number = versions
            .iter()
            .filter(|notes| notes.meeting_id == version.meeting_id)
            .map(|notes| notes.version)
            .max()
            .unwrap_or(0)
            + 1;
        versions.push(EnhancedNotes {
            meeting_id: version.meeting_id.clone(),
            version: number,
            doc: version.doc.clone(),
            markdown: version.markdown.clone(),
            template_id: version.template_id.clone(),
            model: version.model.clone(),
            basis: version.basis.clone(),
            is_user_edited: version.is_user_edited,
            created_at: version.at,
            updated_at: version.at,
        });
        Ok(number)
    }

    async fn save_edit(&self, edit: &VersionEdit) -> Result<bool, DbError> {
        let mut versions = lock(&self.versions);
        let newest = versions
            .iter()
            .filter(|notes| notes.meeting_id == edit.meeting_id)
            .map(|notes| notes.version)
            .max();
        if newest != Some(edit.version) {
            return Ok(false);
        }
        let Some(notes) = versions
            .iter_mut()
            .find(|notes| notes.meeting_id == edit.meeting_id && notes.version == edit.version)
        else {
            return Ok(false);
        };
        notes.doc.clone_from(&edit.doc);
        notes.markdown.clone_from(&edit.markdown);
        notes.is_user_edited = true;
        notes.updated_at = edit.at;
        Ok(true)
    }
}
