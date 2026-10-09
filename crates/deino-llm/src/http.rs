//! The HTTP port: the one way providers reach the network.
//!
//! Providers never hold an HTTP client. They build an [`HttpRequest`] and
//! hand it to an [`HttpTransport`], which the app wires in: one `deino-net`
//! `ScopedClient` per provider's service (FND-12), so each provider reaches
//! only its own hosts, honours the user's switches and offline mode, and
//! every request lands in the egress log.
//!
//! Headers carry API keys, so transports must never log them, and no
//! [`HttpError`] message contains a header or a body.

#[cfg(any(test, feature = "test-support"))]
mod fake;

use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use serde::Serialize;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeHttp;

/// How long a streaming LLM request may take end to end (the legacy limit).
pub const COMPLETION_TIMEOUT: Duration = Duration::from_mins(5);

/// How long a metadata request (model lists, server checks) may take.
pub const METADATA_TIMEOUT: Duration = Duration::from_secs(10);

/// An HTTP method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// `GET`.
    Get,
    /// `POST`.
    Post,
    /// `DELETE`.
    Delete,
}

/// An outgoing request.
#[derive(Clone, Debug, PartialEq)]
pub struct HttpRequest {
    /// The method.
    pub method: Method,
    /// The absolute URL.
    pub url: String,
    /// Header names and values. Values may hold secrets: never log them.
    pub headers: Vec<(&'static str, String)>,
    /// The body, if any.
    pub body: Option<Vec<u8>>,
    /// How long the whole exchange, body included, may take.
    pub timeout: Duration,
}

impl HttpRequest {
    /// A `GET` with the metadata timeout.
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: Method::Get,
            url: url.into(),
            headers: Vec::new(),
            body: None,
            timeout: METADATA_TIMEOUT,
        }
    }

    /// A request with a JSON body and the completion timeout.
    pub fn json(method: Method, url: impl Into<String>, body: &impl Serialize) -> Self {
        // Serializing plain data structs into a Vec cannot fail; an empty
        // body would only make the provider answer 400.
        let body = serde_json::to_vec(body).unwrap_or_default();
        Self {
            method,
            url: url.into(),
            headers: vec![("content-type", "application/json".to_owned())],
            body: Some(body),
            timeout: COMPLETION_TIMEOUT,
        }
    }

    /// Adds a header.
    #[must_use]
    pub fn header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }

    /// Replaces the timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The value of the header `name`, if set.
    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// A response body as it arrives.
pub type BodyStream = BoxStream<'static, Result<Vec<u8>, HttpError>>;

/// A response: status and headers now, the body as a stream.
pub struct HttpResponse {
    /// The HTTP status code.
    pub status: u16,
    /// Header names (lowercase) and values.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: BodyStream,
}

impl HttpResponse {
    /// Whether the status is 2xx.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The value of the header `name`, if present.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Reads the whole body.
    pub async fn bytes(mut self) -> Result<Vec<u8>, HttpError> {
        let mut body = Vec::new();
        while let Some(chunk) = self.body.next().await {
            body.extend_from_slice(&chunk?);
        }
        Ok(body)
    }
}

/// A transport failure. Messages never include headers or bodies.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HttpError {
    /// No connection could be made (server down, DNS, TLS).
    #[error("could not connect: {0}")]
    Connect(String),
    /// The request did not finish within its timeout.
    #[error("the request timed out")]
    Timeout,
    /// The connection broke while sending or receiving.
    #[error("the connection failed: {0}")]
    Transport(String),
}

/// Sends HTTP requests on the app's behalf.
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Sends `request` and returns as soon as the status and headers arrive.
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_requests_carry_a_content_type_and_the_completion_timeout() {
        let request = HttpRequest::json(Method::Post, "https://example.test", &[1, 2])
            .header("x-api-key", "secret");

        assert_eq!(request.body.as_deref(), Some(b"[1,2]".as_slice()));
        assert_eq!(
            request.header_value("Content-Type"),
            Some("application/json")
        );
        assert_eq!(request.header_value("x-api-key"), Some("secret"));
        assert_eq!(request.timeout, COMPLETION_TIMEOUT);
    }
}
