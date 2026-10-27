//! `link_speakers` (order 47): named speakers join the meeting as
//! attendees before the notes are written (50), so the notes see everyone
//! who spoke and their basis already holds them. Linked later, by
//! `update_entities` alone, every meeting's fresh notes said "attendees
//! changed" the moment processing finished.
//!
//! It runs after `identify_speakers` (35) names the speakers. Its work is
//! idempotent; `update_entities` (90) links again for speakers named since.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::entities::EntityUpdater;
use deino_domain::{MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "link_speakers";
/// Where it runs: after the speakers are named, before the notes.
const ORDER: u16 = 47;

/// Adds the meeting's named speakers to its attendees.
pub struct LinkSpeakersStage {
    updater: Arc<dyn EntityUpdater>,
}

impl LinkSpeakersStage {
    /// The stage over the entity updater.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            updater: Arc::clone(&ports.entities),
        }
    }
}

crate::registry::register_stage!(LinkSpeakersStage);

#[async_trait]
impl PipelineStage for LinkSpeakersStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let linked = tokio::select! {
            linked = self.updater.link_speakers(&meeting) => linked,
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
        };
        match linked {
            Ok(count) => {
                log::info!(target: "pipeline", "meeting {meeting}: {count} speakers linked before notes");
                Ok(StageOutcome::Done)
            }
            Err(error) if error.is_retryable() => Err(StageError::retryable_from(&error)),
            Err(error) => Err(StageError::Permanent(error.into())),
        }
    }
}

#[cfg(test)]
mod link_speakers_tests;
