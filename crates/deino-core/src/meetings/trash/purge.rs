//! [`TrashPurger`]: the loop that empties expired meetings out of the trash
//! at launch and every few hours after, until it is cancelled at exit.
//!
//! Other housekeeping that counts in days rides on the same schedule
//! ([`Housekeeping`], added with [`TrashPurger::with`]): PRV-01's retention
//! sweep runs just before each purge, so a meeting it moves to the trash is
//! counted from that moment like any other.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::TrashService;

/// How often the purge looks for expired meetings. Retention is counted in
/// days, so a few hours of lateness never matters.
pub const PURGE_INTERVAL: Duration = Duration::from_hours(6);

/// Work the purge loop runs before each purge.
#[async_trait]
pub trait Housekeeping: Send + Sync {
    /// Does the work once. It logs its own failures: the loop goes on.
    async fn tidy(&self);
}

/// Runs [`TrashService::purge_expired`] on a schedule.
pub struct TrashPurger {
    trash: Arc<TrashService>,
    before: Vec<Arc<dyn Housekeeping>>,
    cancel: CancellationToken,
}

impl TrashPurger {
    /// The purger, and the token that stops it.
    pub fn new(trash: Arc<TrashService>) -> (Self, CancellationToken) {
        let cancel = CancellationToken::new();
        (
            Self {
                trash,
                before: Vec::new(),
                cancel: cancel.clone(),
            },
            cancel,
        )
    }

    /// This purger, also running `job` before each purge.
    #[must_use]
    pub fn with(mut self, job: Arc<dyn Housekeeping>) -> Self {
        self.before.push(job);
        self
    }

    /// Runs the housekeeping and purges now, then every [`PURGE_INTERVAL`],
    /// until cancelled.
    pub async fn run(self) {
        loop {
            for job in &self.before {
                job.tidy().await;
            }
            self.purge().await;
            tokio::select! {
                () = self.cancel.cancelled() => break,
                () = tokio::time::sleep(PURGE_INTERVAL) => {}
            }
        }
        log::debug!(target: "meetings", "trash purge stopped");
    }

    /// One purge, logged.
    async fn purge(&self) {
        match self.trash.purge_expired().await {
            Ok(outcome) if !outcome.changed.is_empty() => {
                log::info!(target: "meetings", "purged {} expired meetings", outcome.changed.len());
            }
            Ok(_) => {}
            Err(error) => log::warn!(target: "meetings", "the trash purge failed: {error}"),
        }
    }
}
