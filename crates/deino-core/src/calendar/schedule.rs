//! When calendars sync: every [`REFRESH_INTERVAL`], and soon after a source
//! reports a change (macOS Calendar's change notification) or the Mac wakes
//! (its timers paused while it slept).
//!
//! The app spawns [`SyncScheduler::run`] once and owns its
//! [`CancellationToken`]; adapters hold [`SyncTriggers`], which never block,
//! so a notification callback on any thread can nudge the scheduler.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::CalendarService;

/// How often calendars sync without a nudge.
pub const REFRESH_INTERVAL: Duration = Duration::from_mins(2);

/// Change notifications arrive in bursts (one per changed calendar); the
/// scheduler waits this long after the first, then syncs once for all.
pub const SETTLE_DELAY: Duration = Duration::from_secs(1);

/// Nudges waiting before the scheduler gets to them; more are dropped,
/// since one sync covers them all.
const TRIGGER_CAPACITY: usize = 16;

/// Why the scheduler should sync early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncTrigger {
    /// A source's data changed.
    SourceChanged,
    /// The Mac woke from sleep.
    Woke,
}

/// Nudges the scheduler. Cheap to clone; never blocks.
#[derive(Clone, Debug)]
pub struct SyncTriggers(mpsc::Sender<SyncTrigger>);

impl SyncTriggers {
    /// Asks for a sync soon. A full queue already holds a sync, so the
    /// nudge is dropped.
    pub fn send(&self, trigger: SyncTrigger) {
        // Full or closed: a sync is already queued, or the app is exiting.
        self.0.try_send(trigger).ok();
    }
}

/// Runs calendar syncs on the schedule.
pub struct SyncScheduler {
    service: Arc<CalendarService>,
    triggers: mpsc::Receiver<SyncTrigger>,
    cancel: CancellationToken,
}

impl SyncScheduler {
    /// A scheduler for `service`, the handle that nudges it, and the token
    /// that stops it.
    pub fn new(service: Arc<CalendarService>) -> (Self, SyncTriggers, CancellationToken) {
        let (sender, triggers) = mpsc::channel(TRIGGER_CAPACITY);
        let cancel = CancellationToken::new();
        let scheduler = Self {
            service,
            triggers,
            cancel: cancel.clone(),
        };
        (scheduler, SyncTriggers(sender), cancel)
    }

    /// Syncs at once, then on every tick or nudge, until cancelled.
    pub async fn run(mut self) {
        loop {
            self.service.sync_if_enabled().await;
            if !self.wait().await {
                break;
            }
        }
        log::debug!(target: "calendar", "calendar sync scheduler stopped");
    }

    /// Waits for the next tick or nudge; false when cancelled.
    async fn wait(&mut self) -> bool {
        tokio::select! {
            () = self.cancel.cancelled() => false,
            () = tokio::time::sleep(REFRESH_INTERVAL) => true,
            Some(trigger) = self.triggers.recv() => {
                log::debug!(target: "calendar", "calendar sync nudged: {trigger:?}");
                self.settle().await
            }
        }
    }

    /// Lets a burst of nudges finish, then drops them all; false when cancelled.
    async fn settle(&mut self) -> bool {
        tokio::select! {
            () = self.cancel.cancelled() => return false,
            () = tokio::time::sleep(SETTLE_DELAY) => {}
        }
        while self.triggers.try_recv().is_ok() {}
        true
    }
}
