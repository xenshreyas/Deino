//! Model file downloads: resumable, size-capped, stall-guarded and verified.
//!
//! [`Downloader`] is the port model managers depend on; [`HttpDownloader`]
//! implements it over any [`NetClient`](crate::NetClient), in the app a
//! [`ScopedClient`](crate::ScopedClient) from
//! [`Net::approved_download`](crate::Net::approved_download), so downloads
//! are confined to the model host, logged, and still work in offline mode.
//!
//! Bytes go to `<destination>.part`. A later attempt resumes it with a
//! `Range` request; a server that ignores the range restarts it; a `416`
//! whose total equals the part's size means it is already complete. The
//! finished part is checked against the expected size and SHA-256, then
//! renamed over the destination, so a half-written or corrupt file never
//! sits at the destination.

mod digest;
mod http;
mod part;

use std::io;
use std::path::PathBuf;

use async_trait::async_trait;
use deino_domain::{ErrorClass, ErrorCode};
use tokio_util::sync::CancellationToken;

pub use digest::{InvalidDigest, Sha256Digest};
pub use http::{DEFAULT_STALL_TIMEOUT, HttpDownloader};

use crate::{NetError, Status};

/// One file to download.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadRequest {
    /// Where the file comes from.
    pub url: String,
    /// Where the verified file goes; the partial file sits next to it.
    pub destination: PathBuf,
    /// The exact size, when the catalog pins it.
    pub size: Option<u64>,
    /// The most bytes accepted, whatever the server claims.
    pub max_bytes: u64,
    /// The expected SHA-256, when the catalog pins it.
    pub sha256: Option<Sha256Digest>,
}

/// A finished download.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Downloaded {
    /// The verified file.
    pub path: PathBuf,
    /// Its size in bytes.
    pub bytes: u64,
    /// The bytes a partial file already held when this attempt began.
    pub resumed_from: u64,
}

/// Receives progress while a download runs.
pub trait DownloadProgress: Send + Sync {
    /// `received` bytes of `total` (when known) are on disk.
    fn progress(&self, received: u64, total: Option<u64>);
}

impl<F: Fn(u64, Option<u64>) + Send + Sync> DownloadProgress for F {
    fn progress(&self, received: u64, total: Option<u64>) {
        self(received, total);
    }
}

/// Downloads files.
#[async_trait]
pub trait Downloader: Send + Sync {
    /// Downloads `request.url` to `request.destination`, resuming a partial
    /// file, until done, failed, or `cancel` fires. A cancelled, stalled or
    /// interrupted download keeps its partial file for the next attempt.
    async fn download(
        &self,
        request: &DownloadRequest,
        progress: &dyn DownloadProgress,
        cancel: &CancellationToken,
    ) -> Result<Downloaded, DownloadError>;
}

/// A download failure. Messages are user-safe.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DownloadError {
    /// The request failed or was blocked.
    #[error(transparent)]
    Net(#[from] NetError),
    /// The server answered with a status that is not the file.
    #[error("the download server answered {0}")]
    Status(Status),
    /// The server's range answer does not match the partial file.
    #[error("the download server sent the wrong part of the file")]
    BadRange,
    /// No data arrived for the stall timeout.
    #[error("the download stalled")]
    Stalled,
    /// The server sent more than the request accepts.
    #[error("the file is larger than {max} bytes")]
    TooLarge {
        /// The cap, in bytes.
        max: u64,
    },
    /// The file ended at the wrong size.
    #[error("the file is {actual} bytes, expected {expected}")]
    SizeMismatch {
        /// The pinned size.
        expected: u64,
        /// What arrived.
        actual: u64,
    },
    /// The file's SHA-256 is not the pinned one; it was deleted.
    #[error("the downloaded file failed its integrity check")]
    ChecksumMismatch,
    /// The caller cancelled.
    #[error("the download was cancelled")]
    Cancelled,
    /// The file could not be written.
    #[error("could not write the downloaded file")]
    Io(#[from] io::Error),
}

impl ErrorClass for DownloadError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Net(error) => error.code(),
            Self::Cancelled => ErrorCode::Cancelled,
            Self::Io(_) => ErrorCode::Internal,
            _ => ErrorCode::Network,
        }
    }
}
