//! The downloader port: where model bytes come from.
//!
//! The core decides what to fetch, where to resume and whether the result is
//! trustworthy; a [`ByteSource`] only moves bytes. The app implements it over
//! `deino-net` (`net.approved_download(ServiceId::HfModels)`); tests use an in-memory
//! fake. Nothing in `deino-stt` can reach this port (§4.0).

use async_trait::async_trait;

/// Opens byte streams.
#[async_trait]
pub trait ByteSource: Send + Sync {
    /// Opens `url`, asking to start at byte `offset`. The stream says where
    /// it actually starts: a server may ignore the range and send it all.
    async fn open(&self, url: &str, offset: u64) -> Result<Box<dyn ByteStream>, FetchError>;
}

/// One open transfer.
#[async_trait]
pub trait ByteStream: Send {
    /// The byte offset of the first byte this stream yields.
    fn start(&self) -> u64;

    /// The next chunk, or `None` at the end.
    async fn chunk(&mut self) -> Result<Option<Vec<u8>>, FetchError>;
}

/// A transfer failure.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// The server answered with an unexpected HTTP status.
    #[error("the server answered {0}")]
    Status(u16),
    /// No answer, or no data, within the timeout.
    #[error("the connection stalled")]
    Stalled,
    /// The connection failed.
    #[error("the connection failed")]
    Connection(#[source] Box<dyn std::error::Error + Send + Sync>),
}
