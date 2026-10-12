//! [`MeetingWork`]: what the trash and the merge ask of the post-meeting
//! pipeline. Trashing a meeting stops its running stage, and a merged
//! meeting rebuilds its search index (keyword and semantic) so search finds
//! the absorbed transcripts by meaning too.
//!
//! The pipeline lives in `deino-pipeline`, a crate above this one, so the
//! app implements the port over `PipelineService`. Both calls are best
//! effort: the trash or the merge has already committed when they run, so
//! an implementation logs its failures instead of returning them.

use async_trait::async_trait;
use deino_domain::MeetingId;

/// The pipeline work a meeting change starts or stops.
#[async_trait]
pub trait MeetingWork: Send + Sync {
    /// Stops the stage running or waiting for `meeting`, if there is one.
    /// The stages after it stay pending until the user re-runs them.
    fn cancel(&self, meeting: &MeetingId);

    /// Schedules `meeting`'s `index` stage, which rebuilds its search
    /// documents and embeddings in the background.
    async fn reindex(&self, meeting: &MeetingId);
}

/// No pipeline: for builds and tests that run without one.
pub struct NoMeetingWork;

#[async_trait]
impl MeetingWork for NoMeetingWork {
    fn cancel(&self, _meeting: &MeetingId) {}

    async fn reindex(&self, _meeting: &MeetingId) {}
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedWork;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use deino_domain::MeetingId;

    use super::MeetingWork;

    /// Records which meetings were cancelled and re-indexed.
    #[derive(Default)]
    pub struct RecordedWork {
        cancelled: Mutex<Vec<MeetingId>>,
        reindexed: Mutex<Vec<MeetingId>>,
    }

    impl RecordedWork {
        /// The meetings whose pipeline was cancelled, in call order.
        pub fn cancelled(&self) -> Vec<MeetingId> {
            self.cancelled
                .lock()
                .map(|ids| ids.clone())
                .unwrap_or_default()
        }

        /// The meetings scheduled for re-indexing, in call order.
        pub fn reindexed(&self) -> Vec<MeetingId> {
            self.reindexed
                .lock()
                .map(|ids| ids.clone())
                .unwrap_or_default()
        }
    }

    #[async_trait]
    impl MeetingWork for RecordedWork {
        fn cancel(&self, meeting: &MeetingId) {
            if let Ok(mut ids) = self.cancelled.lock() {
                ids.push(meeting.clone());
            }
        }

        async fn reindex(&self, meeting: &MeetingId) {
            if let Ok(mut ids) = self.reindexed.lock() {
                ids.push(meeting.clone());
            }
        }
    }
}
