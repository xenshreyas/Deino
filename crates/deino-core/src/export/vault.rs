//! Files on disk, behind the [`Vault`] port: what auto-export and saving
//! read and write.
//!
//! Writes are atomic (a temporary file in the same folder, then a rename),
//! so a note open in Obsidian never shows half a file, and a crash leaves
//! the previous note in place.

use std::io::{self, Read};
use std::path::{Path, PathBuf};

use async_trait::async_trait;

/// How much of an existing note is read to find its owner (the frontmatter).
pub const HEAD_BYTES: usize = 4_096;

/// Reads and writes files.
#[async_trait]
pub trait Vault: Send + Sync {
    /// The first [`HEAD_BYTES`] of the file at `path` as text, or `None`
    /// when there is no file.
    async fn head(&self, path: &Path) -> io::Result<Option<String>>;

    /// Writes `bytes` to `path` atomically, creating its folders.
    async fn write(&self, path: &Path, bytes: Vec<u8>) -> io::Result<()>;

    /// True when `path` is a folder that exists.
    async fn is_dir(&self, path: &Path) -> bool;
}

/// [`Vault`] over the file system.
#[derive(Clone, Copy, Debug, Default)]
pub struct FsVault;

#[async_trait]
impl Vault for FsVault {
    async fn head(&self, path: &Path) -> io::Result<Option<String>> {
        let path = path.to_owned();
        blocking(move || read_head(&path)).await
    }

    async fn write(&self, path: &Path, bytes: Vec<u8>) -> io::Result<()> {
        let path = path.to_owned();
        blocking(move || write_atomically(&path, &bytes)).await
    }

    async fn is_dir(&self, path: &Path) -> bool {
        tokio::fs::metadata(path)
            .await
            .is_ok_and(|metadata| metadata.is_dir())
    }
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(io::Error::other)?
}

fn read_head(path: &Path) -> io::Result<Option<String>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut head = Vec::with_capacity(HEAD_BYTES);
    file.take(u64::try_from(HEAD_BYTES).unwrap_or(u64::MAX))
        .read_to_end(&mut head)?;
    Ok(Some(String::from_utf8_lossy(&head).into_owned()))
}

/// Writes through a hidden temporary file beside `path`, then renames it.
pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let folder = path
        .parent()
        .ok_or_else(|| io::Error::other("the path has no folder"))?;
    std::fs::create_dir_all(folder)?;
    let temporary = temporary_beside(path);
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path).inspect_err(|_| {
        // The rename failed; the temporary file is useless now.
        let _removed = std::fs::remove_file(&temporary);
    })
}

/// `.<name>.deino-part` in the same folder (same volume, so rename is atomic).
pub(crate) fn temporary_beside(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.deino-part"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writes_create_folders_and_replace_files_whole() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("2026/10/Standup.md");

        FsVault.write(&path, b"first".to_vec()).await.unwrap();
        FsVault.write(&path, b"second".to_vec()).await.unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
        assert!(!temporary_beside(&path).exists());
        assert!(FsVault.is_dir(&root.path().join("2026")).await);
    }

    #[tokio::test]
    async fn the_head_of_a_missing_file_is_none() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("note.md");
        assert_eq!(FsVault.head(&path).await.unwrap(), None);

        std::fs::write(&path, "x".repeat(HEAD_BYTES * 2)).unwrap();
        assert_eq!(
            FsVault.head(&path).await.unwrap().map(|head| head.len()),
            Some(HEAD_BYTES)
        );
    }
}
