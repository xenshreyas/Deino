//! The note images port and its filesystem adapter.
//!
//! Each meeting has a folder under the app data folder
//! (`<app data>/meetings/<meeting id>/`); its note images live in
//! `images/` inside it and the document refers to them relatively
//! (`images/<uuid>.png`), so notes survive the data folder moving and the
//! asset protocol, scoped to the app data folder, can show them.

use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use deino_domain::MeetingId;

use super::{IMAGES_FOLDER, ImageName};

/// Where a meeting's note images are kept.
#[async_trait]
pub trait NoteImages: Send + Sync {
    /// The meeting's folder, which image sources are relative to. Fails for
    /// an ID that is not a plain folder name.
    fn folder(&self, meeting: &MeetingId) -> io::Result<PathBuf>;

    /// Stores `bytes` as `name` in the meeting's images folder, atomically.
    async fn write(&self, meeting: &MeetingId, name: &ImageName, bytes: &[u8]) -> io::Result<()>;

    /// Moves every image of `from` into `into`'s images folder. Image names
    /// are UUIDs, so nothing is overwritten; moving twice is harmless.
    async fn relocate(&self, from: &MeetingId, into: &MeetingId) -> io::Result<()>;
}

/// [`NoteImages`] on disk, under one root folder.
#[derive(Clone, Debug)]
pub struct FsNoteImages {
    root: PathBuf,
}

impl FsNoteImages {
    /// Keeps each meeting's folder under `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn images(&self, meeting: &MeetingId) -> io::Result<PathBuf> {
        Ok(self.folder(meeting)?.join(IMAGES_FOLDER))
    }
}

#[async_trait]
impl NoteImages for FsNoteImages {
    fn folder(&self, meeting: &MeetingId) -> io::Result<PathBuf> {
        let id = meeting.as_str();
        let plain = !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !plain {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the meeting ID is not a plain folder name",
            ));
        }
        Ok(self.root.join(id))
    }

    async fn write(&self, meeting: &MeetingId, name: &ImageName, bytes: &[u8]) -> io::Result<()> {
        let folder = self.images(meeting)?;
        tokio::fs::create_dir_all(&folder).await?;
        let partial = folder.join(format!(".{name}.part"));
        tokio::fs::write(&partial, bytes).await?;
        tokio::fs::rename(&partial, folder.join(name.to_string())).await
    }

    async fn relocate(&self, from: &MeetingId, into: &MeetingId) -> io::Result<()> {
        let source = self.images(from)?;
        if !tokio::fs::try_exists(&source).await? {
            return Ok(());
        }
        let target = self.images(into)?;
        tokio::fs::create_dir_all(&target).await?;
        let mut entries = tokio::fs::read_dir(&source).await?;
        while let Some(entry) = entries.next_entry().await? {
            tokio::fs::rename(entry.path(), target.join(entry.file_name())).await?;
        }
        remove_empty(&source).await;
        remove_empty(&self.folder(from)?).await;
        Ok(())
    }
}

/// Removes `folder` if it is empty; a folder with anything left stays.
async fn remove_empty(folder: &Path) {
    if let Err(error) = tokio::fs::remove_dir(folder).await {
        log::debug!(target: "notes", "kept {}: {error}", folder.display());
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::notes::ImageKind;

    fn name(n: u128) -> ImageName {
        ImageName::new(Uuid::from_u128(n), ImageKind::Png)
    }

    #[tokio::test]
    async fn images_are_written_into_the_meetings_images_folder() {
        let root = tempfile::tempdir().unwrap();
        let images = FsNoteImages::new(root.path());
        let meeting = MeetingId::new("meeting-a");

        images.write(&meeting, &name(1), b"png").await.unwrap();

        let stored = root
            .path()
            .join("meeting-a/images")
            .join(name(1).to_string());
        assert_eq!(std::fs::read(stored).unwrap(), b"png");
        assert_eq!(
            std::fs::read_dir(root.path().join("meeting-a/images"))
                .unwrap()
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn relocating_moves_every_image_and_drops_the_empty_folder() {
        let root = tempfile::tempdir().unwrap();
        let images = FsNoteImages::new(root.path());
        let (draft, saved) = (
            MeetingId::new("meeting-draft"),
            MeetingId::new("meeting-saved"),
        );
        images.write(&draft, &name(1), b"a").await.unwrap();
        images.write(&saved, &name(2), b"b").await.unwrap();

        images.relocate(&draft, &saved).await.unwrap();
        images.relocate(&draft, &saved).await.unwrap();

        assert!(!root.path().join("meeting-draft").exists());
        assert_eq!(
            std::fs::read_dir(root.path().join("meeting-saved/images"))
                .unwrap()
                .count(),
            2
        );
    }

    #[test]
    fn meeting_ids_that_are_not_plain_names_have_no_folder() {
        let images = FsNoteImages::new("/data/meetings");
        for id in ["", "..", "../x", "a/b", "a\\b", ".hidden", "meeting 1"] {
            assert!(images.folder(&MeetingId::new(id)).is_err(), "{id:?}");
        }
        let folder = images
            .folder(&MeetingId::new("meeting-fixture-standup"))
            .unwrap();
        assert_eq!(folder, Path::new("/data/meetings/meeting-fixture-standup"));
    }
}
