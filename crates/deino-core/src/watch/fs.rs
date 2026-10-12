//! The file system a watched folder is read through: [`WatchFs`] and its
//! `std::fs` adapter [`LocalWatchFs`].

use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::Sighting;
use crate::import::MediaKind;

/// The subfolder imported files move into with "Move to processed".
pub const PROCESSED_FOLDER: &str = "Processed";

/// The highest number tried for a free name in `Processed/`.
const MAX_NUMBERED_NAME: u32 = 10_000;

/// Lists media files and moves imported ones aside.
pub trait WatchFs: Send + Sync {
    /// Whether `path` is a folder that exists.
    fn is_folder(&self, path: &Path) -> bool;

    /// The audio and video files directly inside `folder`: not its
    /// subfolders (so not `Processed/`), and no hidden files.
    fn scan(&self, folder: &Path) -> io::Result<Vec<Sighting>>;

    /// Moves `file` into `folder`/[`PROCESSED_FOLDER`], creating it, with a
    /// numbered name when the name is taken; returns the new path.
    fn move_to_processed(&self, file: &Path, folder: &Path) -> io::Result<PathBuf>;
}

/// The folder dialog Settings → Watched folders opens.
#[async_trait::async_trait]
pub trait FolderPicker: Send + Sync {
    /// The chosen folder, or `None` when the user cancelled.
    async fn pick_folder(&self) -> Option<PathBuf>;
}

/// [`WatchFs`] over the local file system.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalWatchFs;

impl WatchFs for LocalWatchFs {
    fn is_folder(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn scan(&self, folder: &Path) -> io::Result<Vec<Sighting>> {
        let mut found = Vec::new();
        for entry in std::fs::read_dir(folder)? {
            let entry = entry?;
            let path = entry.path();
            if is_hidden(&path) || MediaKind::of(&path).is_none() {
                continue;
            }
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                found.push(Sighting {
                    path,
                    size_bytes: metadata.len(),
                    modified_ms: modified_ms(&metadata),
                });
            }
        }
        Ok(found)
    }

    fn move_to_processed(&self, file: &Path, folder: &Path) -> io::Result<PathBuf> {
        let processed = folder.join(PROCESSED_FOLDER);
        std::fs::create_dir_all(&processed)?;
        let target = free_name(&processed, file)?;
        std::fs::rename(file, &target)?;
        Ok(target)
    }
}

/// A dot file (`.DS_Store`, a partial download's companion).
fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

/// The file's modification time in ms since the epoch (0 when unknown).
fn modified_ms(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or_default()
}

/// `file`'s name inside `dir`, or "name 2.ext", "name 3.ext", … when taken.
fn free_name(dir: &Path, file: &Path) -> io::Result<PathBuf> {
    let name = file
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "not a file"))?;
    let first = dir.join(name);
    if !first.exists() {
        return Ok(first);
    }
    let stem = file.file_stem().unwrap_or(name).to_string_lossy();
    let extension = file
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();
    (2..=MAX_NUMBERED_NAME)
        .map(|number| dir.join(format!("{stem} {number}{extension}")))
        .find(|candidate| !candidate.exists())
        .ok_or_else(|| io::Error::new(io::ErrorKind::AlreadyExists, "no free name"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scan_lists_media_files_only_at_the_top_level() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("call.mp4"), b"1234").unwrap();
        std::fs::write(dir.path().join("memo.M4A"), b"12").unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();
        std::fs::write(dir.path().join(".hidden.mp3"), b"x").unwrap();
        std::fs::create_dir(dir.path().join("Processed")).unwrap();
        std::fs::write(dir.path().join("Processed/old.mp3"), b"x").unwrap();

        let mut names: Vec<(String, u64)> = LocalWatchFs
            .scan(dir.path())
            .unwrap()
            .into_iter()
            .map(|seen| {
                (
                    seen.path.file_name().unwrap().to_string_lossy().into(),
                    seen.size_bytes,
                )
            })
            .collect();
        names.sort();

        assert_eq!(names, [("call.mp4".into(), 4), ("memo.M4A".into(), 2)]);
    }

    #[test]
    fn moving_to_processed_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("call.mp4");
        std::fs::write(&first, b"one").unwrap();
        let moved = LocalWatchFs.move_to_processed(&first, dir.path()).unwrap();
        std::fs::write(&first, b"two").unwrap();

        let again = LocalWatchFs.move_to_processed(&first, dir.path()).unwrap();

        assert_eq!(moved, dir.path().join("Processed/call.mp4"));
        assert_eq!(again, dir.path().join("Processed/call 2.mp4"));
        assert_eq!(std::fs::read(&again).unwrap(), b"two");
        assert!(!first.exists());
    }
}
