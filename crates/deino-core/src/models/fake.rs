//! In-memory model ports for tests (behind `test-support`).

use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;

use super::{ByteSource, ByteStream, DownloadProgress, FetchError, ModelEvents, ModelFileId};

/// How a [`FakeByteSource`] behaves.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FakeServer {
    /// Honours ranges, like Hugging Face.
    #[default]
    Ranged,
    /// Ignores ranges and always sends everything.
    IgnoresRanges,
    /// Sends this many bytes of the requested range, then fails.
    DropsAfter(usize),
    /// Answers, then never sends a byte.
    Stalls,
}

/// Serves one file's bytes in small chunks.
#[derive(Debug)]
pub struct FakeByteSource {
    bytes: Arc<Vec<u8>>,
    server: Mutex<FakeServer>,
    opened_at: Mutex<Vec<u64>>,
}

/// Bytes per chunk the fake sends.
const CHUNK: usize = 1000;

impl FakeByteSource {
    /// A source serving `bytes` the way `server` says.
    pub fn new(bytes: Vec<u8>, server: FakeServer) -> Self {
        Self {
            bytes: Arc::new(bytes),
            server: Mutex::new(server),
            opened_at: Mutex::new(Vec::new()),
        }
    }

    /// Changes the behaviour for the next request.
    pub fn set_server(&self, server: FakeServer) {
        *self.server.lock().unwrap_or_else(PoisonError::into_inner) = server;
    }

    /// The offset every request asked for, in order.
    pub fn requested_offsets(&self) -> Vec<u64> {
        self.opened_at
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[async_trait]
impl ByteSource for FakeByteSource {
    async fn open(&self, _url: &str, offset: u64) -> Result<Box<dyn ByteStream>, FetchError> {
        self.opened_at
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(offset);
        let server = self
            .server
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let start = match server {
            FakeServer::IgnoresRanges => 0,
            _ => offset,
        };
        let from = usize::try_from(start)
            .unwrap_or(usize::MAX)
            .min(self.bytes.len());
        let to = match server {
            FakeServer::DropsAfter(count) => (from + count).min(self.bytes.len()),
            FakeServer::Stalls => from,
            _ => self.bytes.len(),
        };
        Ok(Box::new(FakeStream {
            bytes: Arc::clone(&self.bytes),
            start,
            at: from,
            to,
            fails: matches!(server, FakeServer::DropsAfter(_)),
            stalls: server == FakeServer::Stalls,
        }))
    }
}

struct FakeStream {
    bytes: Arc<Vec<u8>>,
    start: u64,
    at: usize,
    to: usize,
    fails: bool,
    stalls: bool,
}

#[async_trait]
impl ByteStream for FakeStream {
    fn start(&self) -> u64 {
        self.start
    }

    async fn chunk(&mut self) -> Result<Option<Vec<u8>>, FetchError> {
        if self.stalls {
            std::future::pending::<()>().await;
        }
        if self.at >= self.to {
            return if self.fails {
                Err(FetchError::Connection("connection reset".into()))
            } else {
                Ok(None)
            };
        }
        let end = (self.at + CHUNK).min(self.to);
        let chunk = self.bytes.get(self.at..end).unwrap_or_default().to_vec();
        self.at = end;
        Ok(Some(chunk))
    }
}

/// Records model events for assertions.
#[derive(Debug, Default)]
pub struct RecordedModelEvents {
    progress: Mutex<Vec<DownloadProgress>>,
    changed: Mutex<Vec<ModelFileId>>,
}

impl RecordedModelEvents {
    /// Every progress event, in order.
    pub fn progress(&self) -> Vec<DownloadProgress> {
        self.progress
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Every changed file, in order.
    pub fn changed(&self) -> Vec<ModelFileId> {
        self.changed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl ModelEvents for RecordedModelEvents {
    fn progress(&self, progress: &DownloadProgress) {
        self.progress
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(progress.clone());
    }

    fn changed(&self, id: &ModelFileId) {
        self.changed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(id.clone());
    }
}
