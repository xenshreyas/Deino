//! In-memory clip ports for tests: a recording that is always there (or
//! never), files as a set of paths, a cutter that "writes" into that set,
//! and a Finder that remembers what it showed.

use std::collections::{BTreeSet, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use deino_domain::MeetingId;

use super::{ClipCutter, ClipFiles, ClipSource, ClipSources, CutFailed, CutRequest, Finder};
use crate::player::PlayerError;
use crate::testing::lock;

/// Every meeting's recording is `source` (or none).
#[derive(Debug, Default)]
pub struct FakeClipSources {
    source: Option<ClipSource>,
}

impl FakeClipSources {
    /// Every meeting is recorded in `source`.
    pub fn at(source: ClipSource) -> Self {
        Self {
            source: Some(source),
        }
    }
}

#[async_trait]
impl ClipSources for FakeClipSources {
    async fn source(&self, _meeting: &MeetingId) -> Result<Option<ClipSource>, PlayerError> {
        Ok(self.source.clone())
    }
}

/// Files as a set of paths, all under `root` counting as the app's.
#[derive(Debug)]
pub struct FakeClipFiles {
    root: PathBuf,
    files: Mutex<BTreeSet<PathBuf>>,
}

impl FakeClipFiles {
    /// No files yet; paths under `root` are the app's.
    pub fn under(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            files: Mutex::default(),
        }
    }

    /// Adds a file, as a cut or the user would.
    pub fn add(&self, path: impl Into<PathBuf>) {
        lock(&self.files).insert(path.into());
    }

    /// Every file, sorted.
    pub fn paths(&self) -> Vec<PathBuf> {
        lock(&self.files).iter().cloned().collect()
    }
}

#[async_trait]
impl ClipFiles for FakeClipFiles {
    async fn names_in(&self, folder: &Path) -> io::Result<HashSet<String>> {
        let files = lock(&self.files);
        Ok(files
            .iter()
            .filter(|path| path.parent() == Some(folder))
            .filter_map(|path| path.file_name())
            .map(|name| name.to_string_lossy().to_lowercase())
            .collect())
    }

    async fn create_folder(&self, _folder: &Path) -> io::Result<()> {
        Ok(())
    }

    async fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut files = lock(&self.files);
        if !files.remove(from) {
            return Err(io::ErrorKind::NotFound.into());
        }
        files.insert(to.to_path_buf());
        Ok(())
    }

    async fn remove(&self, path: &Path) -> io::Result<()> {
        lock(&self.files).remove(path);
        Ok(())
    }

    async fn is_ours(&self, path: &Path) -> bool {
        path.starts_with(&self.root) && lock(&self.files).contains(path)
    }
}

/// "Cuts" by adding the destination to the files, or fails when told to.
#[derive(Debug)]
pub struct FakeClipCutter {
    files: Arc<FakeClipFiles>,
    fail: bool,
    cuts: Mutex<Vec<CutRequest>>,
}

impl FakeClipCutter {
    /// A cutter that writes into `files`.
    pub fn writing_to(files: Arc<FakeClipFiles>) -> Self {
        Self {
            files,
            fail: false,
            cuts: Mutex::default(),
        }
    }

    /// A cutter whose every cut fails.
    pub fn failing(files: Arc<FakeClipFiles>) -> Self {
        Self {
            fail: true,
            ..Self::writing_to(files)
        }
    }

    /// The cuts asked for, in order.
    pub fn cuts(&self) -> Vec<CutRequest> {
        lock(&self.cuts).clone()
    }
}

#[async_trait]
impl ClipCutter for FakeClipCutter {
    async fn cut(&self, request: &CutRequest) -> Result<(), CutFailed> {
        lock(&self.cuts).push(request.clone());
        if self.fail {
            return Err(CutFailed);
        }
        self.files.add(request.destination.clone());
        Ok(())
    }
}

/// Remembers every file it was asked to show.
#[derive(Debug, Default)]
pub struct FakeFinder {
    shown: Mutex<Vec<PathBuf>>,
}

impl FakeFinder {
    /// What it showed, in order.
    pub fn shown(&self) -> Vec<PathBuf> {
        lock(&self.shown).clone()
    }
}

#[async_trait]
impl Finder for FakeFinder {
    async fn reveal(&self, path: &Path) -> io::Result<()> {
        lock(&self.shown).push(path.to_path_buf());
        Ok(())
    }
}
