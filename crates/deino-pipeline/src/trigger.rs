//! Starts the pipeline when a meeting enters `processing`.
//!
//! [`PipelineTrigger`] observes the meetings lifecycle: when recording
//! stops (or a meeting is reprocessed) the meeting moves to `processing`,
//! and the trigger enqueues it. It holds the pipeline weakly, because the
//! pipeline holds the lifecycle service that holds the trigger.

use std::sync::{Arc, Weak};

use async_trait::async_trait;
use deino_core::meetings::{MeetingStatusObserver, StatusChange};
use deino_domain::{MeetingId, MeetingStatus};

use crate::PipelineService;

/// Enqueues the pipeline for every meeting that enters `processing`.
pub struct PipelineTrigger {
    pipeline: Weak<PipelineService>,
}

impl PipelineTrigger {
    /// A trigger for `pipeline`; subscribe it to the lifecycle service.
    pub fn new(pipeline: &Arc<PipelineService>) -> Self {
        Self {
            pipeline: Arc::downgrade(pipeline),
        }
    }
}

#[async_trait]
impl MeetingStatusObserver for PipelineTrigger {
    async fn status_changed(&self, id: &MeetingId, change: StatusChange) {
        if change.to != MeetingStatus::Processing {
            return;
        }
        let Some(pipeline) = self.pipeline.upgrade() else {
            return;
        };
        if let Err(error) = pipeline.enqueue(id).await {
            // The meeting stays `processing`; the next start enqueues it.
            log::error!(target: "pipeline", "could not enqueue meeting {id}: {error}");
        }
    }
}
