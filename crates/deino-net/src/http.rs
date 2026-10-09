//! The request and response types every [`NetClient`](crate::NetClient) speaks.
//!
//! They are Deino's own, so no caller ever sees `reqwest`: a provider builds
//! a [`Request`], and gets a [`Response`] whose body arrives as a stream.
//! Header values may hold API keys, so neither type prints them, and the
//! egress log never records a header, a path, a query or a body.

mod headers;
mod method;
mod status;

use std::fmt;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

pub use headers::Headers;
pub use method::Method;
pub use status::Status;

use crate::NetError;

/// How long a request may take end to end unless it says otherwise.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_mins(1);

/// The largest body [`Response::bytes`] reads into memory: 16 MiB. Larger
/// transfers are files, which go through the `Downloader`.
pub const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// A response body as it arrives.
pub type BodyStream = BoxStream<'static, Result<Bytes, NetError>>;

/// An outgoing request.
#[derive(Clone)]
pub struct Request {
    /// The method.
    pub method: Method,
    /// The absolute `http` or `https` URL.
    pub url: Url,
    /// The headers. Values may hold secrets: never log them.
    pub headers: Headers,
    /// The body, if any.
    pub body: Option<Bytes>,
    /// How long the whole exchange, body included, may take; `None` for no
    /// limit (downloads, which use a stall timeout instead).
    pub timeout: Option<Duration>,
}

impl Request {
    /// A request without headers or body; fails unless `url` is an absolute
    /// `http` or `https` URL.
    pub fn new(method: Method, url: &str) -> Result<Self, NetError> {
        let url = Url::parse(url).map_err(|_| NetError::InvalidUrl)?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(NetError::InvalidUrl);
        }
        Ok(Self {
            method,
            url,
            headers: Headers::default(),
            body: None,
            timeout: Some(DEFAULT_TIMEOUT),
        })
    }

    /// A `GET` of `url`.
    pub fn get(url: &str) -> Result<Self, NetError> {
        Self::new(Method::Get, url)
    }

    /// A `POST` to `url`.
    pub fn post(url: &str) -> Result<Self, NetError> {
        Self::new(Method::Post, url)
    }

    /// Adds a header.
    #[must_use]
    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Sets a raw body.
    #[must_use]
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sets a JSON body and its content type.
    pub fn json(self, body: &impl Serialize) -> Result<Self, NetError> {
        let encoded = serde_json::to_vec(body).map_err(NetError::Encode)?;
        Ok(self
            .header("content-type", "application/json")
            .body(encoded))
    }

    /// Replaces the end-to-end timeout; `None` for none.
    #[must_use]
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// The host and, when it is not the scheme's default, the port: what the
    /// egress log records about where the request went.
    pub fn authority(&self) -> String {
        let host = self.url.host_str().unwrap_or_default();
        match self.url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_owned(),
        }
    }

    /// The request body's length in bytes.
    pub fn body_len(&self) -> u64 {
        self.body.as_ref().map_or(0, |body| body.len() as u64)
    }
}

impl fmt::Debug for Request {
    /// Prints the method and authority only: paths, queries, headers and
    /// bodies may hold secrets or meeting content.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Request")
            .field("method", &self.method)
            .field("authority", &self.authority())
            .finish_non_exhaustive()
    }
}

/// A response: the status and headers now, the body as a stream.
pub struct Response {
    /// The status.
    pub status: Status,
    /// The headers.
    pub headers: Headers,
    /// The body.
    pub body: BodyStream,
}

impl Response {
    /// A response with an empty body (fakes, and redirects already read).
    pub fn empty(status: Status) -> Self {
        Self {
            status,
            headers: Headers::default(),
            body: futures_util::stream::empty().boxed(),
        }
    }

    /// The value of the header `name`, if present.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    /// The declared body length, if the server sent a valid one.
    pub fn content_length(&self) -> Option<u64> {
        self.header("content-length")?.trim().parse().ok()
    }

    /// Reads the whole body, failing past `limit` bytes.
    pub async fn bytes(mut self, limit: usize) -> Result<Bytes, NetError> {
        let mut body = BytesMut::new();
        while let Some(chunk) = self.body.next().await {
            let chunk = chunk?;
            if body.len() + chunk.len() > limit {
                return Err(NetError::TooLarge { limit });
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body.freeze())
    }

    /// Reads the whole body (at most [`MAX_BODY_BYTES`]) as JSON.
    pub async fn json<T: DeserializeOwned>(self) -> Result<T, NetError> {
        let body = self.bytes(MAX_BODY_BYTES).await?;
        serde_json::from_slice(&body).map_err(NetError::Decode)
    }

    /// This response if its status is 2xx, else [`NetError::Status`].
    pub fn error_for_status(self) -> Result<Self, NetError> {
        if self.status.is_success() {
            Ok(self)
        } else {
            Err(NetError::Status(self.status))
        }
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Response")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_absolute_http_urls_make_requests() {
        assert!(Request::get("https://api.anthropic.com/v1/messages").is_ok());
        assert!(Request::get("http://localhost:11434/api/tags").is_ok());
        assert!(matches!(Request::get("file:///etc/hosts"), Err(NetError::InvalidUrl)));
        assert!(matches!(Request::get("/relative"), Err(NetError::InvalidUrl)));
    }

    #[test]
    fn the_authority_keeps_a_custom_port_and_drops_the_path_and_query() {
        let local = Request::get("http://localhost:11434/api/tags?x=1").unwrap();
        let cloud = Request::get("https://api.openai.com:443/v1/models?key=sk").unwrap();

        assert_eq!(local.authority(), "localhost:11434");
        assert_eq!(cloud.authority(), "api.openai.com");
    }

    #[test]
    fn debug_output_never_shows_headers_paths_or_bodies() {
        let request = Request::post("https://api.anthropic.com/v1/messages?beta=1")
            .unwrap()
            .header("x-api-key", "sk-ant-secret")
            .body("the meeting transcript");

        let printed = format!("{request:?}");

        assert!(printed.contains("api.anthropic.com"));
        for secret in ["sk-ant-secret", "transcript", "/v1/messages", "beta=1"] {
            assert!(!printed.contains(secret), "{printed} leaks {secret}");
        }
    }

    #[test]
    fn json_bodies_carry_their_content_type() {
        let request = Request::post("https://api.openai.com/v1/chat")
            .unwrap()
            .json(&[1, 2])
            .unwrap();

        assert_eq!(request.body.as_deref(), Some(b"[1,2]".as_slice()));
        assert_eq!(request.headers.get("Content-Type"), Some("application/json"));
    }

    #[tokio::test]
    async fn reading_a_body_stops_at_its_limit() {
        let response = Response {
            status: Status::OK,
            headers: Headers::default(),
            body: futures_util::stream::iter([Ok(Bytes::from_static(b"12345"))]).boxed(),
        };

        let read = response.bytes(4).await;

        assert!(matches!(read, Err(NetError::TooLarge { limit: 4 })));
    }
}
