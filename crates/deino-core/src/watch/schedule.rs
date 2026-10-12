//! [`WatchScheduler`]: scans the watched folders every [`SCAN_INTERVAL`],
//! and at once when nudged (a folder was added or a failure retried).
//!
//! Polling rather than file-system events: the 10 s stability rule needs a
//! clock tick anyway, a few `readdir`s every two seconds cost nothing, and
//! it works the same on network shares and external drives, which `FSEvents`
//! reports unreliably. Imports run inside the scan, one at a time, so a long
//! file delays the next scan rather than overlapping it.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::WatchService;

/// How often every watched folder is scanned.
pub const SCAN_INTERVAL: Duration = Duration::from_secs(2);

/// Nudges waiting before the scheduler gets to them; more are dropped,
/// since one scan covers them all.
const NUDGE_CAPACITY: usize = 4;

/// Asks the scheduler to scan soon. Cheap to clone; never blocks.
#[derive(Clone, Debug)]
pub struct WatchNudge(mpsc::Sender<()>);

impl WatchNudge {
    /// Requests a scan; dropped when one is already queued.
    pub fn send(&self) {
        // Full or closed: a scan is already queued, or the app is exiting.
        self.0.try_send(()).ok();
    }
}

/// Runs the scans.
pub struct WatchScheduler {
    service: Arc<WatchService>,
    nudges: mpsc::Receiver<()>,
    cancel: CancellationToken,
}

impl WatchScheduler {
    /// A scheduler for `service`, the handle that nudges it, and the token
    /// that stops it.
    pub fn new(service: Arc<WatchService>) -> (Self, WatchNudge, CancellationToken) {
        let (sender, nudges) = mpsc::channel(NUDGE_CAPACITY);
        let cancel = CancellationToken::new();
        let scheduler = Self {
            service,
            nudges,
            cancel: cancel.clone(),
        };
        (scheduler, WatchNudge(sender), cancel)
    }

    /// Scans on every tick or nudge until cancelled.
    pub async fn run(mut self) {
        while self.wait().await {
            if let Err(error) = self.service.scan().await {
                log::warn!(target: "watch", "scanning watched folders failed: {error}");
            }
        }
        log::debug!(target: "watch", "watched-folder scheduler stopped");
    }

    /// Waits for the next tick or nudge; false when cancelled.
    async fn wait(&mut self) -> bool {
        tokio::select! {
            () = self.cancel.cancelled() => false,
            () = tokio::time::sleep(SCAN_INTERVAL) => true,
            Some(()) = self.nudges.recv() => {
                while self.nudges.try_recv().is_ok() {}
                true
            }
        }
    }
}
