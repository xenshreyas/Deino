//! [`WebhookSignal`]: a domain event the existing services' event ports
//! announce, and [`WebhookSink`], where the taps send it.
//!
//! Event ports are synchronous and must not wait on the network, so a tap
//! only hands the signal to the sink; the app's sink is a queue the
//! [`DispatchLoop`](super::DispatchLoop) drains into deliveries.

use std::fmt;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use deino_domain::{ActionItemId, MeetingId};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::WebhookEvent;

/// Something happened that endpoints may subscribe to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebhookSignal {
    /// What happened.
    pub event: WebhookEvent,
    /// The meeting, when the port knows it.
    pub meeting: Option<MeetingId>,
    /// The action item, for `action_item.*`.
    pub item: Option<ActionItemId>,
}

impl WebhookSignal {
    /// `event` on `meeting`.
    pub fn meeting(event: WebhookEvent, meeting: MeetingId) -> Self {
        Self {
            event,
            meeting: Some(meeting),
            item: None,
        }
    }

    /// `event` on the action item `item`.
    pub fn item(event: WebhookEvent, item: ActionItemId) -> Self {
        Self {
            event,
            meeting: None,
            item: Some(item),
        }
    }
}

/// Where the taps send signals. Never blocks.
pub trait WebhookSink: Send + Sync {
    /// Announces `signal`.
    fn emit(&self, signal: WebhookSignal);
}

/// A sink that drops every signal: wiring without webhooks.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoWebhooks;

impl fmt::Debug for dyn WebhookSink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("WebhookSink")
    }
}

impl WebhookSink for NoWebhooks {
    fn emit(&self, _signal: WebhookSignal) {}
}

/// The app's sink: an unbounded queue, drained by the dispatch loop.
#[derive(Clone, Debug)]
pub struct SignalQueue {
    sender: UnboundedSender<WebhookSignal>,
}

impl SignalQueue {
    /// The queue and its receiving end (for [`DispatchLoop`](super::DispatchLoop)).
    pub fn new() -> (Self, UnboundedReceiver<WebhookSignal>) {
        let (sender, receiver) = unbounded_channel();
        (Self { sender }, receiver)
    }
}

impl WebhookSink for SignalQueue {
    fn emit(&self, signal: WebhookSignal) {
        if self.sender.send(signal).is_err() {
            log::debug!(target: "webhooks", "the dispatch loop has stopped; signal dropped");
        }
    }
}

/// A sink bound after it is handed out: for wiring built before the app
/// handle that holds the real sink is at hand. Drops signals until bound.
#[derive(Debug, Default)]
pub struct LateSink {
    sink: OnceLock<Arc<dyn WebhookSink>>,
}

impl LateSink {
    /// Sends every later signal to `sink`; a second bind is ignored.
    pub fn bind(&self, sink: Arc<dyn WebhookSink>) {
        if self.sink.set(sink).is_err() {
            log::debug!(target: "webhooks", "the late sink was already bound");
        }
    }
}

impl WebhookSink for LateSink {
    fn emit(&self, signal: WebhookSignal) {
        if let Some(sink) = self.sink.get() {
            sink.emit(signal);
        }
    }
}

/// Keeps every signal, for tests.
#[derive(Debug, Default)]
pub struct RecordedSignals {
    signals: Mutex<Vec<WebhookSignal>>,
}

impl RecordedSignals {
    /// Every signal so far, oldest first.
    pub fn signals(&self) -> Vec<WebhookSignal> {
        self.signals
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl WebhookSink for RecordedSignals {
    fn emit(&self, signal: WebhookSignal) {
        self.signals
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(signal);
    }
}
