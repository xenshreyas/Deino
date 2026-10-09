//! An in-memory store for the folder ports.

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{FolderId, MeetingId};

use super::{Folder, FolderMove, FolderRepo, MeetingFolderRepo};
use crate::meetings::Meeting;
use crate::testing::lock;

/// Keeps folders and filings in `Vec`s and honours the [`FolderRepo`] and
/// [`MeetingFolderRepo`] contracts: orderings, cycle refusal and subtree
/// deletion. `meetings_in` answers from the meetings given to
/// [`FakeFolderRepo::with_meetings`], all of them outside the trash.
#[derive(Debug, Default)]
pub struct FakeFolderRepo {
    folders: Mutex<Vec<Folder>>,
    filed: Mutex<Vec<(MeetingId, FolderId)>>,
    meetings: Vec<Meeting>,
}

impl FakeFolderRepo {
    /// A repository that knows `meetings`, for `meetings_in`.
    pub fn with_meetings(meetings: impl IntoIterator<Item = Meeting>) -> Self {
        Self {
            meetings: meetings.into_iter().collect(),
            ..Self::default()
        }
    }

    /// `folder` and every folder below it.
    fn subtree(folders: &[Folder], folder: FolderId) -> Vec<FolderId> {
        let mut found = vec![folder];
        let mut index = 0;
        while let Some(current) = found.get(index).copied() {
            let children = folders
                .iter()
                .filter(|f| f.parent_id == Some(current))
                .map(|f| f.id);
            found.extend(children);
            index += 1;
        }
        found
    }
}

fn by_name(folders: &mut [Folder]) {
    folders.sort_by_key(|folder| (folder.name.to_lowercase(), folder.id));
}

#[async_trait]
impl FolderRepo for FakeFolderRepo {
    async fn create_folder(&self, folder: &Folder) -> Result<(), DbError> {
        lock(&self.folders).push(folder.clone());
        Ok(())
    }

    async fn folder(&self, id: &FolderId) -> Result<Option<Folder>, DbError> {
        Ok(lock(&self.folders)
            .iter()
            .find(|folder| folder.id == *id)
            .cloned())
    }

    async fn folders(&self) -> Result<Vec<Folder>, DbError> {
        let mut folders = lock(&self.folders).clone();
        by_name(&mut folders);
        // A stable sort keeps the name order within each parent.
        folders.sort_by_key(|folder| folder.parent_id);
        Ok(folders)
    }

    async fn children(&self, parent: Option<&FolderId>) -> Result<Vec<Folder>, DbError> {
        let folders = lock(&self.folders);
        let mut children: Vec<Folder> = folders
            .iter()
            .filter(|folder| folder.parent_id.as_ref() == parent)
            .cloned()
            .collect();
        by_name(&mut children);
        Ok(children)
    }

    async fn update_folder(&self, folder: &Folder) -> Result<bool, DbError> {
        let mut folders = lock(&self.folders);
        let Some(stored) = folders.iter_mut().find(|stored| stored.id == folder.id) else {
            return Ok(false);
        };
        let parent_id = stored.parent_id;
        *stored = Folder {
            parent_id,
            ..folder.clone()
        };
        Ok(true)
    }

    async fn move_folder(
        &self,
        id: &FolderId,
        parent: Option<&FolderId>,
        at: DateTime<Utc>,
    ) -> Result<FolderMove, DbError> {
        let mut folders = lock(&self.folders);
        if parent.is_some_and(|parent| Self::subtree(&folders, *id).contains(parent)) {
            return Ok(FolderMove::WouldCycle);
        }
        let Some(folder) = folders.iter_mut().find(|folder| folder.id == *id) else {
            return Ok(FolderMove::NotFound);
        };
        folder.parent_id = parent.copied();
        folder.updated_at = at;
        Ok(FolderMove::Moved)
    }

    async fn delete_folder(&self, id: &FolderId) -> Result<bool, DbError> {
        let mut folders = lock(&self.folders);
        let doomed = Self::subtree(&folders, *id);
        let before = folders.len();
        folders.retain(|folder| !doomed.contains(&folder.id));
        lock(&self.filed).retain(|(_, folder)| !doomed.contains(folder));
        Ok(folders.len() < before)
    }
}

#[async_trait]
impl MeetingFolderRepo for FakeFolderRepo {
    async fn file_meeting(
        &self,
        meeting: &MeetingId,
        folder: &FolderId,
        _at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let mut filed = lock(&self.filed);
        let pair = (meeting.clone(), *folder);
        if !filed.contains(&pair) {
            filed.push(pair);
        }
        Ok(())
    }

    async fn unfile_meeting(
        &self,
        meeting: &MeetingId,
        folder: &FolderId,
    ) -> Result<bool, DbError> {
        let mut filed = lock(&self.filed);
        let before = filed.len();
        filed.retain(|(m, f)| !(m == meeting && f == folder));
        Ok(filed.len() < before)
    }

    async fn meetings_in(&self, folder: &FolderId) -> Result<Vec<Meeting>, DbError> {
        let filed = lock(&self.filed);
        let inside =
            |meeting: &&Meeting| filed.iter().any(|(m, f)| *m == meeting.id && f == folder);
        let mut meetings: Vec<Meeting> = self.meetings.iter().filter(inside).cloned().collect();
        meetings.sort_by(|a, b| (&b.created_at, &b.id).cmp(&(&a.created_at, &a.id)));
        Ok(meetings)
    }

    async fn folders_of(&self, meeting: &MeetingId) -> Result<Vec<Folder>, DbError> {
        let filed = lock(&self.filed);
        let folders = lock(&self.folders);
        let holds = |folder: &&Folder| filed.iter().any(|(m, f)| m == meeting && *f == folder.id);
        let mut found: Vec<Folder> = folders.iter().filter(holds).cloned().collect();
        by_name(&mut found);
        Ok(found)
    }
}
