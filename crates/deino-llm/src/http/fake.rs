//! A scripted [`HttpTransport`] for provider wire tests: no network.

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream;

use super::{HttpError, HttpRequest, HttpResponse, HttpTransport};

/// One scripted reply.
enum Reply {
    Response {
        status: u16,
        headers: Vec<(String, String)>,
        chunks: Vec<Vec<u8>>,
    },
    Failure(HttpError),
}

/// Answers requests with scripted replies, in order, and records every
/// request so tests can assert on URLs, headers and bodies.
///
/// A request with no reply left fails with [`HttpError::Connect`], like a
/// server that is not running.
#[derive(Default)]
pub struct FakeHttp {
    replies: Mutex<VecDeque<Reply>>,
    requests: Mutex<Vec<HttpRequest>>,
}

impl FakeHttp {
    /// A transport with no replies scripted yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues a reply whose body arrives in one chunk.
    #[must_use]
    pub fn reply(self, status: u16, body: impl Into<Vec<u8>>) -> Self {
        self.reply_chunked(status, vec![body.into()])
    }

    /// Queues a reply whose body arrives in the given chunks, as a stream would.
    #[must_use]
    pub fn reply_chunked(self, status: u16, chunks: Vec<Vec<u8>>) -> Self {
        self.push(Reply::Response {
            status,
            headers: Vec::new(),
            chunks,
        })
    }

    /// Queues a reply with headers, for example `retry-after`.
    #[must_use]
    pub fn reply_with_headers(self, status: u16, headers: &[(&str, &str)], body: &str) -> Self {
        let headers = headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        self.push(Reply::Response {
            status,
            headers,
            chunks: vec![body.as_bytes().to_vec()],
        })
    }

    /// Queues a transport failure.
    #[must_use]
    pub fn fail(self, error: HttpError) -> Self {
        self.push(Reply::Failure(error))
    }

    /// Every request sent so far, oldest first.
    pub fn requests(&self) -> Vec<HttpRequest> {
        lock(&self.requests).clone()
    }

    /// The JSON body of the `index`th request.
    ///
    /// # Panics
    ///
    /// When there is no such request or its body is not JSON, which fails the test.
    #[allow(
        clippy::expect_used,
        reason = "test helper: a missing body must fail the test"
    )]
    pub fn json_body(&self, index: usize) -> serde_json::Value {
        let requests = lock(&self.requests);
        let body = requests
            .get(index)
            .and_then(|request| request.body.as_deref())
            .expect("the request exists and has a body");
        serde_json::from_slice(body).expect("the body is JSON")
    }

    fn push(self, reply: Reply) -> Self {
        lock(&self.replies).push_back(reply);
        self
    }
}

#[async_trait]
impl HttpTransport for FakeHttp {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        lock(&self.requests).push(request);
        let reply = lock(&self.replies).pop_front();
        match reply {
            Some(Reply::Response {
                status,
                headers,
                chunks,
            }) => Ok(HttpResponse {
                status,
                headers,
                body: stream::iter(chunks.into_iter().map(Ok)).boxed(),
            }),
            Some(Reply::Failure(error)) => Err(error),
            None => Err(HttpError::Connect("no scripted reply".to_owned())),
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
