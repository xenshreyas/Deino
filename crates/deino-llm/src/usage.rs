//! The usage log: one [`LlmCall`] per provider call, in `llm_calls`.
//!
//! Recording never blocks or fails a call: the gateway hands each record to
//! a [`UsageRecorder`] synchronously (also from `Drop`, for cancelled
//! streams), and [`UsageChannel`] queues it for a [`UsageWriter`] task that
//! inserts it through [`LlmCallRepo`]. Records hold metadata only: route,
//! provider, model, tokens, latency and outcome, never content.

mod repo;

use std::sync::{Arc, Mutex, PoisonError};

use chrono::{DateTime, Utc};
use deino_domain::LlmCallId;
use tokio::sync::mpsc;

pub use self::repo::{LlmCallRepo, SqliteLlmCallRepo};
use crate::{LlmErrorKind, ProviderId, Route, Usage};

/// One provider call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmCall {
    /// The record's ID.
    pub id: LlmCallId,
    /// When the call started.
    pub started_at: DateTime<Utc>,
    /// What it was for.
    pub route: Route,
    /// Who served it.
    pub provider: ProviderId,
    /// The provider's model ID.
    pub model: String,
    /// Token counts (zero when the call failed before any were reported).
    pub usage: Usage,
    /// From the request to the last event.
    pub latency_ms: u64,
    /// How it ended.
    pub outcome: CallOutcome,
}

/// How a call ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallOutcome {
    /// The provider answered completely.
    Ok,
    /// The call failed.
    Error(LlmErrorKind),
    /// The caller stopped waiting.
    Cancelled,
}

/// Takes usage records. Must not block, fail or panic.
pub trait UsageRecorder: Send + Sync {
    /// Records one finished call.
    fn record(&self, call: LlmCall);
}

/// Queues records for a [`UsageWriter`].
#[derive(Clone, Debug)]
pub struct UsageChannel(mpsc::UnboundedSender<LlmCall>);

impl UsageChannel {
    /// A channel and the writer that drains it.
    pub fn new(repo: Arc<dyn LlmCallRepo>) -> (Self, UsageWriter) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self(sender), UsageWriter { receiver, repo })
    }
}

impl UsageRecorder for UsageChannel {
    fn record(&self, call: LlmCall) {
        if self.0.send(call).is_err() {
            log::warn!(target: "llm", "the usage writer has stopped; a usage record was dropped");
        }
    }
}

/// Inserts queued records, one at a time, until every [`UsageChannel`] is gone.
pub struct UsageWriter {
    receiver: mpsc::UnboundedReceiver<LlmCall>,
    repo: Arc<dyn LlmCallRepo>,
}

impl UsageWriter {
    /// Runs until the channel closes. A failed insert is logged and skipped:
    /// usage is informational and must never stop the app.
    pub async fn run(mut self) {
        while let Some(call) = self.receiver.recv().await {
            if let Err(error) = self.repo.insert(&call).await {
                log::warn!(target: "llm", "could not save usage record {}: {error}", call.id);
            }
        }
    }
}

/// Keeps records in memory, for tests.
#[derive(Debug, Default)]
pub struct MemoryUsage(Mutex<Vec<LlmCall>>);

impl MemoryUsage {
    /// Every record so far, oldest first.
    pub fn calls(&self) -> Vec<LlmCall> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl UsageRecorder for MemoryUsage {
    fn record(&self, call: LlmCall) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;

    #[derive(Default)]
    struct Recorded(Mutex<Vec<LlmCall>>);

    #[async_trait::async_trait]
    impl LlmCallRepo for Recorded {
        async fn insert(&self, call: &LlmCall) -> Result<(), deino_db::DbError> {
            self.0.lock().unwrap().push(call.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn the_writer_saves_every_queued_record_then_stops_when_the_channel_closes() {
        let repo = Arc::new(Recorded::default());
        let (channel, writer) = UsageChannel::new(repo.clone());
        let clock = FakeClock::new(at("2026-10-09T09:00:00Z"));
        let call = LlmCall {
            id: LlmCallId::generate(&clock),
            started_at: at("2026-10-09T09:00:00Z"),
            route: Route::Title,
            provider: ProviderId::Anthropic,
            model: "claude-haiku-5-5".into(),
            usage: Usage::default(),
            latency_ms: 420,
            outcome: CallOutcome::Ok,
        };

        channel.record(call.clone());
        drop(channel);
        writer.run().await;

        assert_eq!(repo.0.lock().unwrap().as_slice(), [call]);
    }
}
