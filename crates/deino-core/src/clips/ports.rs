//! What the clip service works with besides its table: where a meeting's
//! recording is, the clip files on disk, the cutter and Finder.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use deino_domain::MeetingId;

use super::{ClipMedia, ClipSource, CutRequest};
use crate::player::{MediaSource, PlayerError, PlayerService};

/// Finds the recording file a meeting's clips are cut from.
#[async_trait]
pub trait ClipSources: Send + Sync {
    /// The screen recording when there is one, else the mixed audio (else a
    /// single track); `None` without a recording on this Mac.
    async fn source(&self, meeting: &MeetingId) -> Result<Option<ClipSource>, PlayerError>;
}

/// The order a clip's source is chosen in, with the media each gives.
const SOURCE_ORDER: [(MediaSource, ClipMedia); 4] = [
    (MediaSource::Video, ClipMedia::Video),
    (MediaSource::Mixed, ClipMedia::Audio),
    (MediaSource::Microphone, ClipMedia::Audio),
    (MediaSource::System, ClipMedia::Audio),
];

#[async_trait]
impl ClipSources for PlayerService {
    async fn source(&self, meeting: &MeetingId) -> Result<Option<ClipSource>, PlayerError> {
        let media = self.media(meeting).await?;
        Ok(SOURCE_ORDER.iter().find_map(|(source, kind)| {
            media.file(*source).map(|file| ClipSource {
                path: PathBuf::from(&file.path),
                media: *kind,
            })
        }))
    }
}

/// Cuts a range of a recording into a clip file.
#[async_trait]
pub trait ClipCutter: Send + Sync {
    /// Writes the request's range of its source to its destination.
    async fn cut(&self, request: &CutRequest) -> Result<(), CutFailed>;
}

/// A cut that failed; the adapter logged why.
#[derive(Debug, thiserror::Error)]
#[error("the recording could not be cut")]
pub struct CutFailed;

/// The clip files on disk.
#[async_trait]
pub trait ClipFiles: Send + Sync {
    /// The lower-cased names of the files in `folder` (none if it is missing).
    async fn names_in(&self, folder: &Path) -> io::Result<HashSet<String>>;

    /// Creates `folder` and its parents.
    async fn create_folder(&self, folder: &Path) -> io::Result<()>;

    /// Renames `from` to `to`.
    async fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Removes the file at `path`; a missing file is not an error.
    async fn remove(&self, path: &Path) -> io::Result<()>;

    /// Whether `path` is a file inside the app's recordings folder (symlinks
    /// resolved).
    async fn is_ours(&self, path: &Path) -> bool;
}

/// Shows a file in Finder.
#[async_trait]
pub trait Finder: Send + Sync {
    /// Opens the file's folder with the file selected.
    async fn reveal(&self, path: &Path) -> io::Result<()>;
}
