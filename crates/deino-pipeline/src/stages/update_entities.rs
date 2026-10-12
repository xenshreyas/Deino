//! `update_entities` (order 90, ORG-04): brings a processed meeting's people
//! and companies up to date through `deino_core::entities::EntityUpdater`:
//! named speakers join the meeting, attendees' and their companies' AI
//! summaries refresh on `entity.summary`, and company logos are fetched
//! once when the user allows it.
//!
//! It runs late, after notes, action items and the index (orders 50–85), so
//! a summary reads the meeting's finished notes and its open items. Its
//! work is idempotent: running it again rewrites the same summaries.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::entities::EntityUpdater;
use deino_domain::{MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "update_entities";
/// Where it runs (PARITY_PLAN §4.2).
const ORDER: u16 = 90;

/// Updates the meeting's people and companies.
pub struct UpdateEntitiesStage {
    updater: Arc<dyn EntityUpdater>,
}

impl UpdateEntitiesStage {
    /// The stage over the entity updater.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            updater: Arc::clone(&ports.entities),
        }
    }
}

crate::registry::register_stage!(UpdateEntitiesStage);

#[async_trait]
impl PipelineStage for UpdateEntitiesStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let updated = tokio::select! {
            updated = self.updater.update_entities(&meeting) => updated,
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
        };
        match updated {
            Ok(_) => Ok(StageOutcome::Done),
            Err(error) if error.is_retryable() => Err(StageError::retryable_from(&error)),
            Err(error) => Err(StageError::Permanent(error.into())),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedEntityUpdates;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::{Mutex, PoisonError};

    use async_trait::async_trait;
    use deino_core::entities::{EntityError, EntityUpdate, EntityUpdater};
    use deino_domain::MeetingId;

    /// An [`EntityUpdater`] for tests: records each meeting, changes nothing.
    #[derive(Debug, Default)]
    pub struct RecordedEntityUpdates {
        meetings: Mutex<Vec<MeetingId>>,
    }

    impl RecordedEntityUpdates {
        /// Every meeting asked about, oldest first.
        pub fn meetings(&self) -> Vec<MeetingId> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl EntityUpdater for RecordedEntityUpdates {
        async fn update_entities(&self, meeting: &MeetingId) -> Result<EntityUpdate, EntityError> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(meeting.clone());
            Ok(EntityUpdate {
                speakers_linked: 0,
                summaries: 0,
            })
        }
    }
}

#[cfg(test)]
mod update_entities_tests;
