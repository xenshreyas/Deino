//! The [`Links`] port: the few file-system facts and changes installing a
//! symlink needs, and [`FsLinks`] over `std::fs`.

use std::io;
use std::path::{Path, PathBuf};

/// What is at a path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Nothing.
    Missing,
    /// A symbolic link to the path it holds.
    Link(PathBuf),
    /// A file or folder.
    File,
}

/// File-system access for installing the CLI.
pub trait Links: Send + Sync {
    /// What is at `path`, without following a link.
    fn inspect(&self, path: &Path) -> io::Result<Entry>;
    /// True when the app can create files in `dir`.
    fn is_writable(&self, dir: &Path) -> bool;
    /// Creates `dir` and its parents.
    fn create_dir_all(&self, dir: &Path) -> io::Result<()>;
    /// Points `link` at `target`, replacing a link already there.
    fn replace_link(&self, target: &Path, link: &Path) -> io::Result<()>;
}

/// [`Links`] over the real file system.
#[derive(Clone, Copy, Debug, Default)]
pub struct FsLinks;

/// The file a writability probe creates and removes.
const PROBE: &str = ".deino-write-probe";

impl Links for FsLinks {
    fn inspect(&self, path: &Path) -> io::Result<Entry> {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                Ok(Entry::Link(std::fs::read_link(path)?))
            }
            Ok(_) => Ok(Entry::File),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Entry::Missing),
            Err(error) => Err(error),
        }
    }

    /// Permission bits cannot say whether this user may write a folder
    /// (`/usr/local/bin` is often root's), so it tries.
    fn is_writable(&self, dir: &Path) -> bool {
        let probe = dir.join(PROBE);
        let created = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
            .is_ok();
        created && std::fs::remove_file(&probe).is_ok()
    }

    fn create_dir_all(&self, dir: &Path) -> io::Result<()> {
        std::fs::create_dir_all(dir)
    }

    fn replace_link(&self, target: &Path, link: &Path) -> io::Result<()> {
        if let Entry::Link(_) = self.inspect(link)? {
            std::fs::remove_file(link)?;
        }
        symlink(target, link)
    }
}

#[cfg(unix)]
fn symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn symlink(_target: &Path, _link: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "installing the command-line tool is supported on macOS",
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn links_are_made_inspected_and_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let (first, second) = (dir.path().join("a"), dir.path().join("b"));
        let link = dir.path().join("deino");

        assert_eq!(FsLinks.inspect(&link).unwrap(), Entry::Missing);
        FsLinks.replace_link(&first, &link).unwrap();
        FsLinks.replace_link(&second, &link).unwrap();

        assert_eq!(FsLinks.inspect(&link).unwrap(), Entry::Link(second));
        assert!(FsLinks.is_writable(dir.path()));
        assert!(!dir.path().join(PROBE).exists());
    }

    #[test]
    fn a_file_is_reported_as_one_and_a_missing_folder_is_not_writable() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("deino");
        std::fs::write(&file, "#!/bin/sh\n").unwrap();

        assert_eq!(FsLinks.inspect(&file).unwrap(), Entry::File);
        assert!(!FsLinks.is_writable(&dir.path().join("missing")));
    }
}
