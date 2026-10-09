//! The temporary `reqwest` implementation of the gateway's [`HttpTransport`].
//!
//! FND-12 replaces it with `deino-net`'s `ScopedClient` (service registry,
//! egress log, offline mode); nothing else in the app or `deino-llm` changes
//! then. Until that lands this is the one place the gateway touches
//! `reqwest`, isolated behind the port. It never logs a header or a body.

#![allow(
    clippy::disallowed_types,
    reason = "temporary HTTP adapter until FND-12's deino-net owns the only client (PARITY_PLAN §7.2)"
)]

use async_trait::async_trait;
use deino_llm::{HttpError, HttpRequest, HttpResponse, HttpTransport, Method};
use futures_util::StreamExt;

/// Sends the gateway's requests with one shared `reqwest` client.
#[derive(Clone, Debug, Default)]
pub(crate) struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    /// A transport with a fresh connection pool.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        let method = match request.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Delete => reqwest::Method::DELETE,
        };
        let mut builder = self
            .client
            .request(method, &request.url)
            .timeout(request.timeout);
        for (name, value) in request.headers {
            builder = builder.header(name, value);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        let response = builder.send().await.map_err(transport_error)?;
        Ok(HttpResponse {
            status: response.status().as_u16(),
            headers: headers(&response),
            body: response
                .bytes_stream()
                .map(|chunk| chunk.map(|bytes| bytes.to_vec()).map_err(transport_error))
                .boxed(),
        })
    }
}

fn headers(response: &reqwest::Response) -> Vec<(String, String)> {
    response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned()))
        })
        .collect()
}

/// The error's kind, without the URL's query or any body.
fn transport_error(error: reqwest::Error) -> HttpError {
    if error.is_timeout() {
        HttpError::Timeout
    } else if error.is_connect() {
        HttpError::Connect(error.without_url().to_string())
    } else {
        HttpError::Transport(error.without_url().to_string())
    }
}
