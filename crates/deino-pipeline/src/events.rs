//! Pipeline progress: the event port and the event itself.
//!
//! The runner reports every change of a stage's status as a
//! [`PipelineProgress`] through the [`ProgressSink`] port; a long stage
//! may also report how far it has got ([`StageFraction`]). The app
//! implements the port with the typed `PipelineProgress` Tauri event, which
//! the meeting page's status strip listens to; tests use
//! [`RecordedProgress`]. The `jobs` table is the truth: a missed event costs
//! the UI a moment of staleness, never data.

use deino_domain::{JobStatus, MeetingId, StageId};

#[cfg(doc)]
use crate::StageFraction;

/// A stage of a meeting's pipeline changed status, or a running stage got
/// further.
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineProgress {
    /// The meeting being processed.
    pub meeting_id: MeetingId,
    /// The stage whose status changed.
    pub stage: StageId,
    /// Its new status.
    pub status: JobStatus,
    /// How far the running stage is, 0.0 to 1.0, when it reports that
    /// ([`StageFraction`]); `None` on a plain status change.
    pub fraction: Option<f32>,
}

impl PipelineProgress {
    /// A plain status change, with no fraction.
    pub fn status(meeting_id: MeetingId, stage: StageId, status: JobStatus) -> Self {
        Self {
            meeting_id,
            stage,
            status,
            fraction: None,
        }
    }

    /// The fraction as a whole percent, 0 to 100, rounded down so a stage
    /// reads 100% only once it is done.
    pub fn percent(&self) -> Option<u8> {
        self.fraction.map(whole_percent)
    }
}

/// `fraction` (clamped to 0.0–1.0) as a whole percent, rounded down.
pub(crate) fn whole_percent(fraction: f32) -> u8 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0.0..=100.0 first; NaN becomes 0"
    )]
    let percent = (fraction.clamp(0.0, 1.0) * 100.0).floor() as u8;
    percent
}

/// Receives pipeline progress. Implementations must not block: they are
/// called from the runner's tasks.
pub trait ProgressSink: Send + Sync {
    /// A stage changed status.
    fn progress(&self, progress: &PipelineProgress);
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedProgress;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::{Mutex, PoisonError};
    use std::time::Duration;

    use tokio::sync::Notify;

    use super::{PipelineProgress, ProgressSink};

    /// How long [`RecordedProgress::wait_for`] waits before failing the test.
    const WAIT_TIMEOUT: Duration = Duration::from_secs(5);

    /// Keeps every event in order, and lets a test wait for one.
    #[derive(Debug, Default)]
    pub struct RecordedProgress {
        events: Mutex<Vec<PipelineProgress>>,
        arrived: Notify,
    }

    impl RecordedProgress {
        /// Every event so far, oldest first.
        pub fn events(&self) -> Vec<PipelineProgress> {
            self.events
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        /// Waits until `done` holds for the events so far.
        ///
        /// # Panics
        ///
        /// After a few seconds without `done` holding, which fails the test.
        pub async fn wait_for(&self, done: impl Fn(&[PipelineProgress]) -> bool) {
            let waited = tokio::time::timeout(WAIT_TIMEOUT, self.until(done)).await;
            assert!(
                waited.is_ok(),
                "timed out waiting for progress; got {:?}",
                self.events()
            );
        }

        async fn until(&self, done: impl Fn(&[PipelineProgress]) -> bool) {
            loop {
                // Enabled before checking, so an event in between still wakes us.
                let arrived = self.arrived.notified();
                tokio::pin!(arrived);
                arrived.as_mut().enable();
                if done(&self.events()) {
                    return;
                }
                arrived.await;
            }
        }
    }

    impl ProgressSink for RecordedProgress {
        fn progress(&self, progress: &PipelineProgress) {
            self.events
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(progress.clone());
            self.arrived.notify_waiters();
        }
    }
}
