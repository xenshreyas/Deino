//! Usage metering: every provider call becomes exactly one [`LlmCall`].
//!
//! A [`Meter`] starts when the gateway sends a request. Wrapped around the
//! provider's stream, it adds up usage events and records the call when the
//! stream stops, fails, ends without a stop (truncated) or is dropped
//! (cancelled), whichever comes first.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use chrono::{DateTime, Utc};
use deino_domain::{Clock, LlmCallId};
use futures_util::{Stream, StreamExt};

use crate::{
    CallOutcome, CompletionEvent, EventStream, LlmCall, LlmError, LlmErrorKind, Route, Target,
    Usage, UsageRecorder,
};

/// One call being measured.
pub(super) struct Meter {
    recorder: Arc<dyn UsageRecorder>,
    clock: Arc<dyn Clock>,
    route: Route,
    target: Target,
    started_at: DateTime<Utc>,
    usage: Usage,
}

impl Meter {
    /// Starts measuring a call to `target` for `route`.
    pub(super) fn start(
        recorder: Arc<dyn UsageRecorder>,
        clock: Arc<dyn Clock>,
        route: Route,
        target: &Target,
    ) -> Self {
        let started_at = clock.now();
        Self {
            recorder,
            clock,
            route,
            target: target.clone(),
            started_at,
            usage: Usage::default(),
        }
    }

    /// Records the call as failed with `error`.
    pub(super) fn fail(self, error: &LlmError) {
        self.finish(outcome_of(error));
    }

    /// Records a call that finished with `usage` (non-streamed work, like embeddings).
    pub(super) fn succeed(mut self, usage: Usage) {
        self.usage = self.usage.merge(usage);
        self.finish(CallOutcome::Ok);
    }

    /// Measures `stream` until it ends or is dropped.
    pub(super) fn wrap(self, stream: EventStream) -> EventStream {
        Metered {
            inner: stream,
            meter: Some(self),
        }
        .boxed()
    }

    fn finish(self, outcome: CallOutcome) {
        let now = self.clock.now();
        let elapsed = (now - self.started_at).num_milliseconds();
        let call = LlmCall {
            id: LlmCallId::generate(self.clock.as_ref()),
            started_at: self.started_at,
            route: self.route,
            provider: self.target.provider,
            model: self.target.model,
            usage: self.usage,
            latency_ms: u64::try_from(elapsed).unwrap_or_default(),
            outcome,
        };
        log::debug!(target: "llm", "{} call {} ended: {:?}", call.route, call.id, call.outcome);
        self.recorder.record(call);
    }
}

fn outcome_of(error: &LlmError) -> CallOutcome {
    match error {
        LlmError::Cancelled => CallOutcome::Cancelled,
        error => CallOutcome::Error(error.kind()),
    }
}

/// A stream that reports itself to its [`Meter`].
struct Metered {
    inner: EventStream,
    meter: Option<Meter>,
}

impl Metered {
    fn observe(&mut self, item: Option<&Result<CompletionEvent, LlmError>>) {
        let outcome = match item {
            Some(Ok(CompletionEvent::Usage(usage))) => {
                if let Some(meter) = &mut self.meter {
                    meter.usage = meter.usage.merge(*usage);
                }
                return;
            }
            Some(Ok(CompletionEvent::Stop(_))) => CallOutcome::Ok,
            Some(Ok(_)) => return,
            Some(Err(error)) => outcome_of(error),
            None => CallOutcome::Error(LlmErrorKind::Truncated),
        };
        if let Some(meter) = self.meter.take() {
            meter.finish(outcome);
        }
    }
}

impl Stream for Metered {
    type Item = Result<CompletionEvent, LlmError>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let poll = this.inner.poll_next_unpin(context);
        if let Poll::Ready(item) = &poll {
            this.observe(item.as_ref());
        }
        poll
    }
}

impl Drop for Metered {
    fn drop(&mut self) {
        if let Some(meter) = self.meter.take() {
            meter.finish(CallOutcome::Cancelled);
        }
    }
}
