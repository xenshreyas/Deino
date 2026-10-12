//! [`TrashPurger`]: the loop that empties expired meetings out of the trash
//! at launch and every few hours after, until it is cancelled at exit.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::TrashService;

/// How often the purge looks for expired meetings. Retention is counted in
/// days, so a few hours of lateness never matters.
pub const PURGE_INTERVAL: Duration = Duration::from_hours(6);

/// Runs [`TrashService::purge_expired`] on a schedule.
pub struct TrashPurger {
    trash: Arc<TrashService>,
    cancel: CancellationToken,
}

impl TrashPurger {
    /// The purger, and the token that stops it.
    pub fn new(trash: Arc<TrashService>) -> (Self, CancellationToken) {
        let cancel = CancellationToken::new();
        (
            Self {
                trash,
                cancel: cancel.clone(),
            },
            cancel,
        )
    }

    /// Purges now, then every [`PURGE_INTERVAL`], until cancelled.
    pub async fn run(self) {
        loop {
            match self.trash.purge_expired().await {
                Ok(outcome) if !outcome.changed.is_empty() => {
                    log::info!(target: "meetings", "purged {} expired meetings", outcome.changed.len());
                }
                Ok(_) => {}
                Err(error) => log::warn!(target: "meetings", "the trash purge failed: {error}"),
            }
            tokio::select! {
                () = self.cancel.cancelled() => break,
                () = tokio::time::sleep(PURGE_INTERVAL) => {}
            }
        }
        log::debug!(target: "meetings", "trash purge stopped");
    }
}
