//! The note images port and its filesystem adapter.
//!
//! Each meeting has a folder under the app data folder
//! (`<app data>/meetings/<meeting id>/`); its note images live in
//! `images/` inside it and the document refers to them relatively
//! (`images/<uuid>.png`), so notes survive the data folder moving and the
//! asset protocol, scoped to the app data folder, can show them.

use std::io;
use std::path::PathBuf;

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

    #[test]
    fn meeting_ids_that_are_not_plain_names_have_no_folder() {
        let images = FsNoteImages::new("/data/meetings");
        for id in ["", "..", "../x", "a/b", "a\\b", ".hidden", "meeting 1"] {
            assert!(images.folder(&MeetingId::new(id)).is_err(), "{id:?}");
        }
        let folder = images
            .folder(&MeetingId::new("meeting-fixture-standup"))
            .unwrap();
        assert_eq!(
            folder,
            std::path::Path::new("/data/meetings/meeting-fixture-standup")
        );
    }
}
