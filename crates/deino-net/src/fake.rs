//! [`FakeNetClient`]: a scripted [`NetClient`] for tests, behind
//! `test-support`. Nothing leaves the process.

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;

use crate::{Headers, NetClient, NetError, Request, Response, Status};

/// A scripted answer.
#[derive(Clone, Debug)]
pub struct FakeReply {
    status: Status,
    headers: Vec<(String, String)>,
    body: Bytes,
}

impl FakeReply {
    /// `200 OK` with `body`.
    pub fn ok(body: impl Into<Bytes>) -> Self {
        Self {
            status: Status::OK,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    /// `status` with an empty body.
    pub fn status(status: Status) -> Self {
        Self {
            status,
            ..Self::ok(Bytes::new())
        }
    }

    /// The same answer with another status, for error bodies (`400` with
    /// an OAuth error, say).
    #[must_use]
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// A redirect to `location`.
    pub fn redirect(status: Status, location: &str) -> Self {
        Self::status(status).header("location", location)
    }

    /// Adds a response header.
    #[must_use]
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

/// Answers each request with the next scripted [`FakeReply`] (`200 OK` with
/// an empty body once the script runs out) and records every request.
#[derive(Debug, Default)]
pub struct FakeNetClient {
    replies: Mutex<VecDeque<FakeReply>>,
    requests: Mutex<Vec<Request>>,
}

impl FakeNetClient {
    /// Queues the answer to a later request.
    pub fn reply(&self, reply: FakeReply) {
        lock(&self.replies).push_back(reply);
    }

    /// Every request received, in order.
    pub fn requests(&self) -> Vec<Request> {
        lock(&self.requests).clone()
    }
}

#[async_trait]
impl NetClient for FakeNetClient {
    async fn send(&self, request: Request) -> Result<Response, NetError> {
        lock(&self.requests).push(request);
        let reply = lock(&self.replies)
            .pop_front()
            .unwrap_or_else(|| FakeReply::ok(Bytes::new()));
        let headers: Headers = reply.headers.into_iter().collect();
        let body = futures_util::stream::iter([Ok(reply.body)]).boxed();
        Ok(Response {
            status: reply.status,
            headers,
            body,
        })
    }
}

/// A panic in another test thread cannot leave the script half-updated.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
