//! [`FsClipFiles`]: clip files on the local disk, confined to the
//! recordings folder.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use super::ClipFiles;

/// Clip files under `root` (`<app data>/recordings`).
#[derive(Clone, Debug)]
pub struct FsClipFiles {
    root: PathBuf,
}

impl FsClipFiles {
    /// The files under `root`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl ClipFiles for FsClipFiles {
    async fn names_in(&self, folder: &Path) -> io::Result<HashSet<String>> {
        let mut names = HashSet::new();
        let mut entries = match tokio::fs::read_dir(folder).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(names),
            Err(error) => return Err(error),
        };
        while let Some(entry) = entries.next_entry().await? {
            names.insert(entry.file_name().to_string_lossy().to_lowercase());
        }
        Ok(names)
    }

    async fn create_folder(&self, folder: &Path) -> io::Result<()> {
        tokio::fs::create_dir_all(folder).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        tokio::fs::rename(from, to).await
    }

    async fn remove(&self, path: &Path) -> io::Result<()> {
        match tokio::fs::remove_file(path).await {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }

    async fn is_ours(&self, path: &Path) -> bool {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn only_files_inside_the_recordings_folder_are_ours() {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path().join("recordings");
        let clip = root.join("meeting-a/clips/Intro.m4a");
        tokio::fs::create_dir_all(clip.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&clip, b"aac").await.unwrap();
        let outside = scratch.path().join("elsewhere.m4a");
        tokio::fs::write(&outside, b"aac").await.unwrap();
        let files = FsClipFiles::new(root.clone());

        assert!(files.is_ours(&clip).await);
        assert!(!files.is_ours(&outside).await);
        assert!(
            !files
                .is_ours(&root.join("meeting-a/clips/../../../elsewhere.m4a"))
                .await
        );
        assert!(!files.is_ours(&root.join("meeting-a")).await);
    }

    #[tokio::test]
    async fn names_are_listed_lower_case_and_a_missing_folder_has_none() {
        let scratch = tempfile::tempdir().unwrap();
        let files = FsClipFiles::new(scratch.path().into());
        tokio::fs::write(scratch.path().join("Intro.m4a"), b"aac")
            .await
            .unwrap();

        let names = files.names_in(scratch.path()).await.unwrap();
        assert!(names.contains("intro.m4a"));
        assert!(
            files
                .names_in(&scratch.path().join("missing"))
                .await
                .unwrap()
                .is_empty()
        );
        files
            .remove(&scratch.path().join("missing.m4a"))
            .await
            .unwrap();
    }
}
