//! Metering: turning one request's lifetime into one [`EgressEntry`].
//!
//! An [`Exchange`] is started for each request a scoped client sends. It is
//! finished at once when policy blocks the request or the connection fails;
//! otherwise it rides along inside the response body as a [`MeteredBody`],
//! which counts bytes as they arrive and records the entry when the body
//! ends, fails, or is dropped unread.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::{Stream, StreamExt};

use super::{EgressEntry, EgressLog, EgressOutcome};
use crate::{BodyStream, NetError};

/// One request being recorded.
#[derive(Debug)]
pub(crate) struct Exchange {
    log: Arc<EgressLog>,
    entry: EgressEntry,
}

impl Exchange {
    /// Starts recording `entry` (its outcome and counts are filled in later).
    pub(crate) fn new(log: Arc<EgressLog>, entry: EgressEntry) -> Self {
        Self { log, entry }
    }

    /// Records the request as `outcome`. A blocked request sent nothing.
    pub(crate) fn finish(mut self, outcome: EgressOutcome) {
        if outcome == EgressOutcome::Blocked {
            self.entry.bytes_up = 0;
        }
        self.entry.outcome = outcome;
        self.log.record(self.entry);
    }

    /// Wraps a response body so the entry is recorded when it ends.
    pub(crate) fn meter(mut self, status: u16, body: BodyStream) -> MeteredBody {
        self.entry.status = Some(status);
        MeteredBody {
            inner: body,
            exchange: Some(self),
        }
    }
}

/// A response body that records its exchange when it is done.
pub(crate) struct MeteredBody {
    inner: BodyStream,
    exchange: Option<Exchange>,
}

impl MeteredBody {
    fn settle(&mut self, outcome: EgressOutcome) {
        if let Some(exchange) = self.exchange.take() {
            exchange.finish(outcome);
        }
    }
}

impl Stream for MeteredBody {
    type Item = Result<Bytes, NetError>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let polled = self.inner.poll_next_unpin(context);
        match &polled {
            Poll::Ready(Some(Ok(chunk))) => {
                if let Some(exchange) = self.exchange.as_mut() {
                    exchange.entry.bytes_down += chunk.len() as u64;
                }
            }
            Poll::Ready(Some(Err(_))) => self.settle(EgressOutcome::Failed),
            Poll::Ready(None) => self.settle(EgressOutcome::Sent),
            Poll::Pending => {}
        }
        polled
    }
}

impl Drop for MeteredBody {
    /// A body dropped unread (a redirect, a cancelled download) still counts
    /// as sent, with the bytes read so far.
    fn drop(&mut self) {
        self.settle(EgressOutcome::Sent);
    }
}
