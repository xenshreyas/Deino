//! [`DispatchLoop`]: the one background task that turns signals into
//! deliveries and attempts each delivery when it is due, while the app runs.
//!
//! At launch it puts attempts a quit cut off back in the queue, so pending
//! deliveries survive restarts. It then sleeps until the next delivery is
//! due (at most [`IDLE_WAKE`]) or a signal arrives, until it is cancelled at
//! exit.

use std::time::Duration;

use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

use super::{WebhookService, WebhookSignal};

/// The longest the loop sleeps without looking at the queue.
pub const IDLE_WAKE: Duration = Duration::from_mins(1);

/// Runs the webhooks' deliveries in the background.
pub struct DispatchLoop {
    service: WebhookService,
    signals: Option<UnboundedReceiver<WebhookSignal>>,
    cancel: CancellationToken,
}

impl DispatchLoop {
    /// The loop over `service`, fed by `signals`, and the token that stops it.
    pub fn new(
        service: WebhookService,
        signals: UnboundedReceiver<WebhookSignal>,
    ) -> (Self, CancellationToken) {
        let cancel = CancellationToken::new();
        let this = Self {
            service,
            signals: Some(signals),
            cancel: cancel.clone(),
        };
        (this, cancel)
    }

    /// Recovers interrupted attempts, then works until cancelled.
    pub async fn run(mut self) {
        self.recover().await;
        loop {
            if let Err(error) = self.service.dispatcher().run_due().await {
                log::warn!(target: "webhooks", "the delivery pass failed: {error}");
            }
            let wait = self.wait().await;
            tokio::select! {
                () = self.cancel.cancelled() => break,
                signal = next_signal(&mut self.signals) => self.take(signal).await,
                () = tokio::time::sleep(wait) => {}
            }
        }
        log::debug!(target: "webhooks", "the dispatch loop stopped");
    }

    /// Puts attempts a quit cut off back in the queue.
    async fn recover(&self) {
        let ports = &self.service.ports;
        match ports.deliveries.recover(ports.clock.now()).await {
            Ok(0) => {}
            Ok(count) => log::info!(target: "webhooks", "resuming {count} interrupted deliveries"),
            Err(error) => log::warn!(target: "webhooks", "could not resume deliveries: {error}"),
        }
    }

    /// Queues `signal` and anything else already waiting.
    async fn take(&mut self, signal: Option<WebhookSignal>) {
        let Some(first) = signal else {
            self.signals = None;
            return;
        };
        let mut batch = vec![first];
        if let Some(receiver) = &mut self.signals {
            while let Ok(more) = receiver.try_recv() {
                batch.push(more);
            }
        }
        for signal in &batch {
            if let Err(error) = self.service.accept(signal).await {
                log::warn!(target: "webhooks", "could not queue {}: {error}", signal.event.as_str());
            }
        }
    }

    /// How long to sleep: until the next delivery is due, at most [`IDLE_WAKE`].
    async fn wait(&self) -> Duration {
        let ports = &self.service.ports;
        match ports.deliveries.next_due().await {
            Ok(Some(next)) => (next - ports.clock.now())
                .to_std()
                .unwrap_or(Duration::ZERO)
                .min(IDLE_WAKE),
            Ok(None) | Err(_) => IDLE_WAKE,
        }
    }
}

/// The next signal, or never once the queue has closed.
async fn next_signal(
    signals: &mut Option<UnboundedReceiver<WebhookSignal>>,
) -> Option<WebhookSignal> {
    match signals {
        Some(receiver) => receiver.recv().await,
        None => std::future::pending().await,
    }
}
