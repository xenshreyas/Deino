//! [`MediaFiles`]: the recording files on disk, confined to the recordings
//! folder. [`FsMediaFiles`] is the real one.

use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

/// The recording files the player may serve and copy.
#[async_trait]
pub trait MediaFiles: Send + Sync {
    /// Whether `path` is a file inside the recordings folder (symlinks
    /// resolved), so the asset protocol, scoped to that folder, serves it.
    async fn playable(&self, path: &Path) -> bool;

    /// Copies the file at `from` to `to`, replacing what is there.
    async fn copy(&self, from: &Path, to: &Path) -> io::Result<()>;
}

/// Recording files on the local disk under `root`
/// (`<app data>/recordings`).
#[derive(Clone, Debug)]
pub struct FsMediaFiles {
    root: PathBuf,
}

impl FsMediaFiles {
    /// The files under `root`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl MediaFiles for FsMediaFiles {
    async fn playable(&self, path: &Path) -> bool {
        let (Ok(root), Ok(file)) = (
            tokio::fs::canonicalize(&self.root).await,
            tokio::fs::canonicalize(path).await,
        ) else {
            return false;
        };
        let is_file = tokio::fs::metadata(&file)
            .await
            .is_ok_and(|metadata| metadata.is_file());
        is_file && file.starts_with(root)
    }

    async fn copy(&self, from: &Path, to: &Path) -> io::Result<()> {
        tokio::fs::copy(from, to).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn only_files_inside_the_recordings_folder_are_playable() {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path().join("recordings");
        tokio::fs::create_dir_all(root.join("meeting-a"))
            .await
            .unwrap();
        let inside = root.join("meeting-a").join("mic.m4a");
        let outside = scratch.path().join("elsewhere.m4a");
        tokio::fs::write(&inside, b"audio").await.unwrap();
        tokio::fs::write(&outside, b"audio").await.unwrap();
        let files = FsMediaFiles::new(root.clone());

        assert!(files.playable(&inside).await);
        assert!(!files.playable(&outside).await);
        assert!(
            !files
                .playable(&root.join("meeting-a/../../elsewhere.m4a"))
                .await
        );
        assert!(!files.playable(&root.join("meeting-a")).await);
        assert!(!files.playable(&root.join("missing.m4a")).await);
    }

    #[tokio::test]
    async fn a_copy_replaces_the_destination() {
        let scratch = tempfile::tempdir().unwrap();
        let (from, to) = (scratch.path().join("a"), scratch.path().join("b"));
        tokio::fs::write(&from, b"new").await.unwrap();
        tokio::fs::write(&to, b"old contents").await.unwrap();

        FsMediaFiles::new(scratch.path().into())
            .copy(&from, &to)
            .await
            .unwrap();

        assert_eq!(tokio::fs::read(&to).await.unwrap(), b"new");
    }
}
