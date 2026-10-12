//! [`ClipService`]: cutting, listing, renaming, revealing and deleting clips.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use deino_domain::{ClipId, Clock, MeetingId};

use super::naming::{safe_stem, unique_file_name};
use super::{
    Clip, ClipCutter, ClipError, ClipFiles, ClipRename, ClipRepo, ClipRow, ClipSource, ClipSources,
    CutRequest, Finder, NewClip,
};

/// The folder of clips beside a meeting's recording files.
pub const CLIPS_FOLDER: &str = "clips";

/// What the clip service works with.
pub struct ClipDeps {
    /// The `clips` table.
    pub repo: Arc<dyn ClipRepo>,
    /// Where each meeting's recording is.
    pub sources: Arc<dyn ClipSources>,
    /// The clip files.
    pub files: Arc<dyn ClipFiles>,
    /// Cuts a range into a file.
    pub cutter: Arc<dyn ClipCutter>,
    /// Shows a clip in Finder.
    pub finder: Arc<dyn Finder>,
    /// Stamps new clips and renames.
    pub clock: Arc<dyn Clock>,
}

/// The clips of meetings' recordings.
pub struct ClipService {
    deps: ClipDeps,
}

impl ClipService {
    /// The service over its ports.
    pub fn new(deps: ClipDeps) -> Self {
        Self { deps }
    }

    /// Cuts the range into `<recording folder>/clips/<name>.m4a` (`.mp4` for
    /// a screen recording) and saves it.
    pub async fn create(&self, new: NewClip) -> Result<Clip, ClipError> {
        let name = checked_name(&new.name)?;
        if new.end <= new.start {
            return Err(ClipError::EmptyRange);
        }
        let source = self.source(&new.meeting_id).await?;
        let destination = self.free_path(&source, &name).await?;
        let request = CutRequest {
            source: source.path.clone(),
            destination: destination.clone(),
            start: new.start,
            end: new.end,
            media: source.media,
        };
        self.deps.cutter.cut(&request).await?;
        let row = ClipRow {
            id: ClipId::generate(self.deps.clock.as_ref()),
            meeting_id: new.meeting_id,
            name,
            start: new.start,
            end: new.end,
            media: source.media,
            path: destination.to_string_lossy().into_owned(),
            at: self.deps.clock.now(),
        };
        self.save(&row, &destination).await?;
        self.deps
            .repo
            .find(&row.id)
            .await?
            .ok_or(ClipError::NotFound)
    }

    /// A meeting's clips, in time order.
    pub async fn list(&self, meeting: &MeetingId) -> Result<Vec<Clip>, ClipError> {
        Ok(self.deps.repo.list_for_meeting(meeting).await?)
    }

    /// Every clip of every meeting not in the trash, newest first.
    pub async fn list_all(&self) -> Result<Vec<Clip>, ClipError> {
        Ok(self.deps.repo.list_all().await?)
    }

    /// Renames the clip and its file (to a free, safe name).
    pub async fn rename(&self, id: &ClipId, name: &str) -> Result<Clip, ClipError> {
        let name = checked_name(name)?;
        let clip = self.existing(id).await?;
        let from = PathBuf::from(&clip.path);
        let folder = from.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut taken = self.taken_in(&folder).await?;
        let own = file_name(&from);
        taken.remove(&own);
        let to = folder.join(unique_file_name(
            &safe_stem(&name),
            clip.media.extension(),
            &taken,
        ));
        if to != from {
            self.deps
                .files
                .rename(&from, &to)
                .await
                .map_err(ClipError::File)?;
        }
        let change = ClipRename {
            name,
            path: to.to_string_lossy().into_owned(),
            at: self.deps.clock.now(),
        };
        if !self.deps.repo.rename(id, &change).await? {
            return Err(ClipError::NotFound);
        }
        self.existing(id).await
    }

    /// Deletes the clip and its file.
    pub async fn delete(&self, id: &ClipId) -> Result<(), ClipError> {
        let clip = self.existing(id).await?;
        self.deps.repo.delete(id).await?;
        let path = PathBuf::from(&clip.path);
        if self.deps.files.is_ours(&path).await {
            self.deps
                .files
                .remove(&path)
                .await
                .map_err(ClipError::File)?;
        }
        Ok(())
    }

    /// Shows the clip's file in Finder, if it is inside the app's folders.
    pub async fn reveal(&self, id: &ClipId) -> Result<(), ClipError> {
        let clip = self.existing(id).await?;
        let path = PathBuf::from(&clip.path);
        if !self.deps.files.is_ours(&path).await {
            return Err(ClipError::FileMissing);
        }
        self.deps
            .finder
            .reveal(&path)
            .await
            .map_err(ClipError::File)
    }

    /// The clip, or [`ClipError::NotFound`].
    async fn existing(&self, id: &ClipId) -> Result<Clip, ClipError> {
        self.deps.repo.find(id).await?.ok_or(ClipError::NotFound)
    }

    /// The recording to cut from.
    async fn source(&self, meeting: &MeetingId) -> Result<ClipSource, ClipError> {
        self.deps
            .sources
            .source(meeting)
            .await?
            .ok_or(ClipError::NoRecording)
    }

    /// A free path for a clip called `name` beside the source.
    async fn free_path(&self, source: &ClipSource, name: &str) -> Result<PathBuf, ClipError> {
        let recording = source
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let folder = recording.join(CLIPS_FOLDER);
        self.deps
            .files
            .create_folder(&folder)
            .await
            .map_err(ClipError::File)?;
        let taken = self.taken_in(&folder).await?;
        Ok(folder.join(unique_file_name(
            &safe_stem(name),
            source.media.extension(),
            &taken,
        )))
    }

    /// The lower-cased file names used in `folder`: on disk or by a row.
    async fn taken_in(&self, folder: &Path) -> Result<HashSet<String>, ClipError> {
        let mut taken = self
            .deps
            .files
            .names_in(folder)
            .await
            .map_err(ClipError::File)?;
        let prefix = format!("{}{}", folder.to_string_lossy(), std::path::MAIN_SEPARATOR);
        for path in self.deps.repo.paths_like(&prefix).await? {
            taken.insert(file_name(Path::new(&path)));
        }
        Ok(taken)
    }

    /// Saves the row; a failed save removes the cut file again.
    async fn save(&self, row: &ClipRow, file: &Path) -> Result<(), ClipError> {
        if let Err(error) = self.deps.repo.insert(row).await {
            self.deps.files.remove(file).await.ok();
            return Err(error.into());
        }
        Ok(())
    }
}

/// The trimmed name, or [`ClipError::EmptyName`].
fn checked_name(name: &str) -> Result<String, ClipError> {
    let name = name.trim();
    if name.is_empty() {
        Err(ClipError::EmptyName)
    } else {
        Ok(name.to_owned())
    }
}

/// The lower-cased file name of `path`.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}
