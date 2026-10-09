//! [`NetByteSource`]: `deino_core::models::ByteSource` over `deino-net`.
//!
//! In the app the client is `net.approved_download(ServiceId::HfModels)`
//! (FND-12): it reaches only Hugging Face and its CDNs, follows their
//! redirects hop by hop, records every request in the egress log, and still
//! works in offline mode because the user starts every model download.
//!
//! It only moves bytes: resume decisions, size limits, timeouts and SHA-256
//! verification live in `deino_core::models::download`. The STT engines never
//! see it, so transcription has no route to the network.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::models::{ByteSource, ByteStream, FetchError};
use deino_net::{BodyStream, NetClient, NetError, Request, Response, Status};
use futures_util::StreamExt;

/// Fetches model files, with `Range` requests for resumes.
pub struct NetByteSource {
    client: Arc<dyn NetClient>,
}

impl NetByteSource {
    /// A source over `client`, in the app a model-download scoped client.
    pub fn new(client: Arc<dyn NetClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ByteSource for NetByteSource {
    async fn open(&self, url: &str, offset: u64) -> Result<Box<dyn ByteStream>, FetchError> {
        // No end-to-end timeout: the core times out a stalled body per chunk.
        let mut request = Request::get(url).map_err(connection)?.timeout(None);
        if offset > 0 {
            request = request.header("range", format!("bytes={offset}-"));
        }
        let response = self.client.send(request).await.map_err(connection)?;
        let start = match response.status {
            Status::OK => 0,
            Status::PARTIAL_CONTENT => range_start(&response).unwrap_or(u64::MAX),
            status => return Err(FetchError::Status(status.as_u16())),
        };
        Ok(Box::new(NetStream {
            body: response.body,
            start,
        }))
    }
}

/// The first byte of a `206` response: `Content-Range: bytes <start>-<end>/<size>`.
fn range_start(response: &Response) -> Option<u64> {
    let range = response.header("content-range")?.strip_prefix("bytes ")?;
    range.split_once('-')?.0.trim().parse().ok()
}

fn connection(error: NetError) -> FetchError {
    FetchError::Connection(Box::new(error))
}

/// One response body.
struct NetStream {
    body: BodyStream,
    start: u64,
}

#[async_trait]
impl ByteStream for NetStream {
    fn start(&self) -> u64 {
        self.start
    }

    async fn chunk(&mut self) -> Result<Option<Vec<u8>>, FetchError> {
        match self.body.next().await {
            None => Ok(None),
            Some(chunk) => Ok(Some(chunk.map_err(connection)?.to_vec())),
        }
    }
}
