//! In-memory stand-ins for the export ports (feature `test-support`).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;

use super::{
    Clipboard, ClipboardContent, ExportError, FolderDialog, SaveDialog, SaveRequest, UrlOpener,
    Vault, vault::HEAD_BYTES,
};

/// Locks a test double's state; a poisoned lock means a test already failed.
fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A [`Vault`] in memory: files by path, and folders that "exist".
#[derive(Debug, Default)]
pub struct FakeVault {
    files: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
    folders: Mutex<Vec<PathBuf>>,
    fail_writes: Mutex<bool>,
}

impl FakeVault {
    /// A vault where `folder` exists.
    pub fn with_folder(folder: impl Into<PathBuf>) -> Self {
        let vault = Self::default();
        locked(&vault.folders).push(folder.into());
        vault
    }

    /// Puts a file in place, as if the user had written it.
    pub fn put(&self, path: impl Into<PathBuf>, text: &str) {
        locked(&self.files).insert(path.into(), text.as_bytes().to_vec());
    }

    /// Makes every later write fail, like an unplugged drive.
    pub fn fail_writes(&self) {
        *locked(&self.fail_writes) = true;
    }

    /// The file at `path`, as text.
    pub fn read(&self, path: impl AsRef<Path>) -> Option<String> {
        locked(&self.files)
            .get(path.as_ref())
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }

    /// Every file's path, sorted.
    pub fn paths(&self) -> Vec<PathBuf> {
        locked(&self.files).keys().cloned().collect()
    }
}

#[async_trait]
impl Vault for FakeVault {
    async fn head(&self, path: &Path) -> io::Result<Option<String>> {
        Ok(self
            .read(path)
            .map(|text| text.chars().take(HEAD_BYTES).collect()))
    }

    async fn write(&self, path: &Path, bytes: Vec<u8>) -> io::Result<()> {
        if *locked(&self.fail_writes) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "the drive is gone"));
        }
        locked(&self.files).insert(path.to_owned(), bytes);
        Ok(())
    }

    async fn is_dir(&self, path: &Path) -> bool {
        locked(&self.folders)
            .iter()
            .any(|folder| path.starts_with(folder))
    }
}

/// A save or folder dialog that answers with a fixed path (or cancels), and
/// remembers what it was asked.
#[derive(Debug, Default)]
pub struct ScriptedDialog {
    answer: Option<PathBuf>,
    requests: Mutex<Vec<SaveRequest>>,
}

impl ScriptedDialog {
    /// A dialog where the user picks `path`; `None` cancels.
    pub fn answering(answer: Option<PathBuf>) -> Self {
        Self {
            answer,
            requests: Mutex::default(),
        }
    }

    /// What the save dialog was asked, in order.
    pub fn requests(&self) -> Vec<SaveRequest> {
        locked(&self.requests).clone()
    }
}

#[async_trait]
impl SaveDialog for ScriptedDialog {
    async fn save_path(&self, request: SaveRequest) -> Option<PathBuf> {
        locked(&self.requests).push(request);
        self.answer.clone()
    }
}

#[async_trait]
impl FolderDialog for ScriptedDialog {
    async fn pick_folder(&self) -> Option<PathBuf> {
        self.answer.clone()
    }
}

/// A clipboard that keeps what was copied.
#[derive(Debug, Default)]
pub struct RecordedClipboard {
    copies: Mutex<Vec<ClipboardContent>>,
}

impl RecordedClipboard {
    /// Everything copied, in order.
    pub fn copies(&self) -> Vec<ClipboardContent> {
        locked(&self.copies).clone()
    }
}

#[async_trait]
impl Clipboard for RecordedClipboard {
    async fn write(&self, content: ClipboardContent) -> Result<(), ExportError> {
        locked(&self.copies).push(content);
        Ok(())
    }
}

/// A URL opener that keeps what it was asked to open.
#[derive(Debug, Default)]
pub struct RecordedOpener {
    urls: Mutex<Vec<String>>,
}

impl RecordedOpener {
    /// Every URL opened, in order.
    pub fn urls(&self) -> Vec<String> {
        locked(&self.urls).clone()
    }
}

#[async_trait]
impl UrlOpener for RecordedOpener {
    async fn open(&self, url: &str) -> Result<(), ExportError> {
        locked(&self.urls).push(url.to_owned());
        Ok(())
    }
}
