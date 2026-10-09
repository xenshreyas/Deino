//! [`ReqwestNetClient`]: the app's one HTTP client, over `reqwest`.
//!
//! This is the only place in Deino that builds an HTTP client (the clippy
//! `disallowed-types` rule bans it everywhere else). It is the root client
//! that [`Net`](crate::Net) wraps: nothing outside the composition root
//! should hold it directly, because it checks no policy and logs nothing.
//!
//! It never follows redirects itself. [`ScopedClient`](crate::ScopedClient)
//! follows them, so every hop is checked against the service's hosts and
//! written to the egress log.

#![allow(
    clippy::disallowed_types,
    reason = "FND-12: deino-net owns the only HTTP client"
)]

use std::time::Duration;

use async_trait::async_trait;
use futures_util::{StreamExt, TryStreamExt};

use crate::{Headers, Method, NetClient, NetError, Request, Response, Status};

/// How long to wait for a TCP and TLS connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Identifies Deino to the services it calls when the app names no version.
const DEFAULT_USER_AGENT: &str = "Deino";

/// Sends requests over the network, with no policy of its own.
#[derive(Clone, Debug)]
pub struct ReqwestNetClient {
    client: reqwest::Client,
}

impl ReqwestNetClient {
    /// Builds the client: native TLS, no redirects, a connect timeout, and
    /// the `User-Agent` `Deino`.
    pub fn new() -> Result<Self, NetError> {
        Self::with_user_agent(DEFAULT_USER_AGENT)
    }

    /// Like [`ReqwestNetClient::new`], sending `user_agent` (the app sends
    /// `Deino/<app version>`).
    pub fn with_user_agent(user_agent: &str) -> Result<Self, NetError> {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(CONNECT_TIMEOUT)
            .tcp_nodelay(true)
            .build()
            .map_err(|error| NetError::Connect(Box::new(error)))?;
        Ok(Self { client })
    }

    fn build(&self, request: Request) -> reqwest::RequestBuilder {
        let mut builder = self.client.request(method(request.method), request.url);
        for (name, value) in request.headers.iter() {
            builder = builder.header(name, value);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        if let Some(timeout) = request.timeout {
            builder = builder.timeout(timeout);
        }
        builder
    }
}

#[async_trait]
impl NetClient for ReqwestNetClient {
    async fn send(&self, request: Request) -> Result<Response, NetError> {
        let response = self.build(request).send().await.map_err(classify)?;
        let headers: Headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| Some((name.as_str(), value.to_str().ok()?)))
            .collect();
        Ok(Response {
            status: Status::new(response.status().as_u16()),
            headers,
            body: response.bytes_stream().map_err(classify).boxed(),
        })
    }
}

fn method(method: Method) -> reqwest::Method {
    match method {
        Method::Get => reqwest::Method::GET,
        Method::Head => reqwest::Method::HEAD,
        Method::Post => reqwest::Method::POST,
        Method::Put => reqwest::Method::PUT,
        Method::Patch => reqwest::Method::PATCH,
        Method::Delete => reqwest::Method::DELETE,
    }
}

/// Maps a reqwest error, first stripping its URL, which may carry a query
/// string with a key in it.
fn classify(error: reqwest::Error) -> NetError {
    let error = error.without_url();
    if error.is_timeout() {
        NetError::Timeout(Box::new(error))
    } else if error.is_connect() {
        NetError::Connect(Box::new(error))
    } else {
        NetError::Transport(Box::new(error))
    }
}
