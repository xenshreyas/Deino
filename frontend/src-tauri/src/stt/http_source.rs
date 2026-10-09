//! The HTTP adapter behind `deino_core::models::ByteSource`.
//!
//! **Temporary (until FND-12):** PARITY_PLAN §4.0 makes `deino-net` the only
//! owner of an HTTP client, with a service registry and an egress log. It is
//! not built yet, so model downloads use this one small adapter, the single
//! exception to the `reqwest::Client` ban outside legacy code. FND-12 replaces
//! it with a `deino-net` client registered as the `hf_models` service.
//!
//! It only moves bytes: resume decisions, size limits, timeouts and SHA-256
//! verification live in `deino_core::models::download`.

#![allow(
    clippy::disallowed_types,
    reason = "FND-12 (deino-net) is not built yet; this is the one temporary HTTP client, for model downloads"
)]

use async_trait::async_trait;
use deino_core::models::{ByteSource, ByteStream, FetchError};
use reqwest::header::{CONTENT_RANGE, RANGE};
use reqwest::{Client, Response, StatusCode};

/// Identifies Deino to the model host.
const USER_AGENT: &str = concat!("Deino/", env!("CARGO_PKG_VERSION"));

/// Fetches model files over HTTPS, with `Range` requests for resumes.
pub struct HttpByteSource {
    client: Client,
}

impl HttpByteSource {
    /// A source with its own HTTP client.
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = Client::builder().user_agent(USER_AGENT).build()?;
        Ok(Self { client })
    }
}

#[async_trait]
impl ByteSource for HttpByteSource {
    async fn open(&self, url: &str, offset: u64) -> Result<Box<dyn ByteStream>, FetchError> {
        let mut request = self.client.get(url);
        if offset > 0 {
            request = request.header(RANGE, format!("bytes={offset}-"));
        }
        let response = request.send().await.map_err(connection)?;
        let start = match response.status() {
            StatusCode::OK => 0,
            StatusCode::PARTIAL_CONTENT => range_start(&response).unwrap_or(u64::MAX),
            status => return Err(FetchError::Status(status.as_u16())),
        };
        Ok(Box::new(HttpStream { response, start }))
    }
}

/// The first byte of a `206` response: `Content-Range: bytes <start>-<end>/<size>`.
fn range_start(response: &Response) -> Option<u64> {
    let header = response.headers().get(CONTENT_RANGE)?.to_str().ok()?;
    let range = header.strip_prefix("bytes ")?;
    range.split_once('-')?.0.trim().parse().ok()
}

fn connection(error: reqwest::Error) -> FetchError {
    FetchError::Connection(Box::new(error))
}

/// One HTTP response body.
struct HttpStream {
    response: Response,
    start: u64,
}

#[async_trait]
impl ByteStream for HttpStream {
    fn start(&self) -> u64 {
        self.start
    }

    async fn chunk(&mut self) -> Result<Option<Vec<u8>>, FetchError> {
        let chunk = self.response.chunk().await.map_err(connection)?;
        Ok(chunk.map(|bytes| bytes.to_vec()))
    }
}
