//! [`MeetingFiles`]: deleting what a meeting keeps on disk once its rows are
//! gone for good, and [`DiskMeetingFiles`], the adapter over the app's data
//! folder.
//!
//! A meeting owns two folders, `recordings/<id>` (the live recorder's
//! tracks and finished files) and `meetings/<id>` (note images, NOT-01).
//! Its `audio_*` paths usually point inside the first; a path anywhere else
//! is the user's own file (an import) and is never deleted.

use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use super::StoredFiles;

/// Deletes meetings' files.
#[async_trait]
pub trait MeetingFiles: Send + Sync {
    /// Deletes `files`' folders and the recordings inside them. Missing
    /// files are not an error.
    async fn delete(&self, files: &StoredFiles) -> io::Result<()>;
}

/// [`MeetingFiles`] over the app's data folder.
#[derive(Clone, Debug)]
pub struct DiskMeetingFiles {
    recordings: PathBuf,
    notes: PathBuf,
}

impl DiskMeetingFiles {
    /// The adapter over `recordings` (`<data>/recordings`) and `notes`
    /// (`<data>/meetings`).
    pub fn new(recordings: PathBuf, notes: PathBuf) -> Self {
        Self { recordings, notes }
    }

    /// Deletes `files`' recordings and the `recordings/<owner>` folder (clips
    /// included), but not the note images in `meetings/<owner>`: what
    /// retention removes when a meeting's audio expires (PRV-01). Paths
    /// outside the recordings folder are the user's own and stay. Missing
    /// files are not an error.
    pub async fn delete_recordings(&self, files: &StoredFiles) -> io::Result<()> {
        for path in self.owned_paths(files) {
            ignore_missing(tokio::fs::remove_file(path).await)?;
        }
        if let Some(id) = owner_folder(files) {
            ignore_missing(tokio::fs::remove_dir_all(self.recordings.join(id)).await)?;
        }
        Ok(())
    }

    /// The recordings inside the app's folders; the user's own files stay.
    fn owned_paths<'a>(&self, files: &'a StoredFiles) -> impl Iterator<Item = &'a Path> {
        let recordings = self.recordings.clone();
        files
            .paths
            .iter()
            .map(Path::new)
            .filter(move |path| inside(path, &recordings))
    }
}

#[async_trait]
impl MeetingFiles for DiskMeetingFiles {
    async fn delete(&self, files: &StoredFiles) -> io::Result<()> {
        self.delete_recordings(files).await?;
        if let Some(id) = owner_folder(files) {
            ignore_missing(tokio::fs::remove_dir_all(self.notes.join(id)).await)?;
        }
        Ok(())
    }
}

/// The folder name `files.owner` owns, unless its ID could leave the roots.
fn owner_folder(files: &StoredFiles) -> Option<&str> {
    let id = files.owner.as_str();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        log::warn!(target: "meetings", "skipped the folders of an unexpected meeting ID");
        return None;
    }
    Some(id)
}

/// True when `path` is absolute and lies under `root` with no `..` step.
fn inside(path: &Path, root: &Path) -> bool {
    use std::path::Component;
    path.is_absolute()
        && path.starts_with(root)
        && !path
            .components()
            .any(|step| matches!(step, Component::ParentDir))
}

/// `result`, with "not found" as success.
fn ignore_missing(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::MeetingId;

    use super::*;

    fn layout() -> (tempfile::TempDir, DiskMeetingFiles) {
        let data = tempfile::tempdir().unwrap();
        let files =
            DiskMeetingFiles::new(data.path().join("recordings"), data.path().join("meetings"));
        (data, files)
    }

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"audio").unwrap();
    }

    #[tokio::test]
    async fn deletes_the_meetings_folders_and_recordings_but_never_the_users_files() {
        let (data, disk) = layout();
        let track = data.path().join("recordings/meeting-1/mic.m4a");
        let image = data.path().join("meetings/meeting-1/images/a.png");
        let imported = data.path().join("Desktop/interview.m4a");
        let neighbour = data.path().join("recordings/meeting-2/mic.m4a");
        for path in [&track, &image, &imported, &neighbour] {
            touch(path);
        }
        let files = StoredFiles {
            meeting: MeetingId::new("meeting-1"),
            owner: MeetingId::new("meeting-1"),
            paths: vec![track.display().to_string(), imported.display().to_string()],
        };

        disk.delete(&files).await.unwrap();

        assert!(!track.exists() && !image.exists());
        assert!(!data.path().join("recordings/meeting-1").exists());
        assert!(
            imported.exists(),
            "a file outside the app's folders is the user's"
        );
        assert!(neighbour.exists());
    }

    #[tokio::test]
    async fn missing_files_and_odd_ids_are_not_errors() {
        let (data, disk) = layout();
        touch(&data.path().join("recordings/x"));
        let odd = StoredFiles {
            meeting: MeetingId::new("meeting-8"),
            owner: MeetingId::new("../recordings"),
            paths: Vec::new(),
        };
        let gone = StoredFiles {
            meeting: MeetingId::new("meeting-9"),
            owner: MeetingId::new("meeting-9"),
            paths: vec![
                data.path()
                    .join("recordings/meeting-9/mic.m4a")
                    .display()
                    .to_string(),
            ],
        };

        disk.delete(&odd).await.unwrap();
        disk.delete(&gone).await.unwrap();

        assert!(
            data.path().join("recordings/x").exists(),
            "`..` never escapes a root"
        );
    }
}
