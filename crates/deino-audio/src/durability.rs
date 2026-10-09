//! Making bytes survive a crash and a power loss.
//!
//! On macOS `fsync` only reaches the drive's cache; `F_FULLFSYNC` asks the
//! drive to flush it too. A rename is durable only once its directory is
//! synced. [`write_atomic`] combines both into the standard
//! "temp file → sync → rename → sync directory" sequence, so a reader sees
//! either the old file or the new one, never a torn mix.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Suffix of the temporary file [`write_atomic`] writes before renaming.
const TEMP_SUFFIX: &str = ".tmp";

/// Flushes `file` all the way to stable storage (`F_FULLFSYNC` on Apple
/// platforms, falling back to `fsync` where the volume does not support it).
pub fn full_fsync(file: &File) -> io::Result<()> {
    #[cfg(target_vendor = "apple")]
    {
        if rustix::fs::fcntl_fullfsync(file).is_ok() {
            return Ok(());
        }
    }
    file.sync_all()
}

/// Syncs a directory so renames and new entries inside it are durable.
pub fn fsync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        full_fsync(&File::open(dir)?)
    }
    #[cfg(not(unix))]
    {
        // Windows cannot open a directory as a file; NTFS journals renames.
        let _ = dir;
        Ok(())
    }
}

/// Replaces `path` with `bytes` atomically and durably.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut temp_name = path.as_os_str().to_owned();
    temp_name.push(TEMP_SUFFIX);
    let temp = Path::new(&temp_name);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(temp)?;
    file.write_all(bytes)?;
    full_fsync(&file)?;
    rename_durably(temp, path)
}

/// Renames `from` to `to` and syncs the directory, so the rename survives a
/// power loss.
pub fn rename_durably(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::rename(from, to)?;
    match to.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => fsync_dir(dir),
        _ => fsync_dir(Path::new(".")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_atomic_write_replaces_the_file_and_leaves_no_temp_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recording.json");
        write_atomic(&path, b"old").unwrap();

        write_atomic(&path, b"new").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        let entries = std::fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(entries, 1);
    }

    #[test]
    fn a_durable_rename_moves_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("mic.001.m4a.part");
        std::fs::write(&from, b"fmp4").unwrap();
        let to = dir.path().join("mic.001.m4a");

        rename_durably(&from, &to).unwrap();

        assert!(!from.exists());
        assert_eq!(std::fs::read(&to).unwrap(), b"fmp4");
    }

    #[test]
    fn full_fsync_accepts_a_regular_file() {
        let file = tempfile::tempfile().unwrap();
        full_fsync(&file).unwrap();
    }
}
