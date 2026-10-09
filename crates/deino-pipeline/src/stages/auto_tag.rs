//! `auto_tag` (order 70, ORG-06): applies the user's existing tags to a
//! processed meeting through `deino_core::tags::AutoTagger`.
//!
//! It runs after notes and titles (orders 50–65), so the classifier reads
//! the summary, and before `auto_folder` (75), so tag rules see its tags.
//! The skip rules (Settings off, tagged by hand, no tags, no content) live
//! in the core service; this stage maps its outcome and errors.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::tags::{AutoTagged, AutoTagger};
use deino_domain::{MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "auto_tag";
/// Where it runs (PARITY_PLAN §4.2).
const ORDER: u16 = 70;

/// Tags the meeting with the auto-tagger.
pub struct AutoTagStage {
    tagger: Arc<dyn AutoTagger>,
}

impl AutoTagStage {
    /// The stage over the auto-tagger.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            tagger: Arc::clone(&ports.auto_tagger),
        }
    }
}

#[async_trait]
impl PipelineStage for AutoTagStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let tagged = tokio::select! {
            tagged = self.tagger.auto_tag(&meeting) => tagged,
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
        };
        match tagged {
            Ok(AutoTagged::Applied(_)) => Ok(StageOutcome::Done),
            Ok(AutoTagged::Skipped(skip)) => Ok(StageOutcome::Skipped(skip.reason())),
            Err(error) if error.is_retryable() => Err(StageError::retryable_from(&error)),
            Err(error) => Err(StageError::Permanent(error.into())),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedAutoTags;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::{Mutex, PoisonError};

    use async_trait::async_trait;
    use deino_core::tags::{AutoTagged, AutoTagger, TagError};
    use deino_domain::MeetingId;

    /// An [`AutoTagger`] for tests: records each meeting and applies nothing.
    #[derive(Debug, Default)]
    pub struct RecordedAutoTags {
        meetings: Mutex<Vec<MeetingId>>,
    }

    impl RecordedAutoTags {
        /// Every meeting asked about, oldest first.
        pub fn meetings(&self) -> Vec<MeetingId> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl AutoTagger for RecordedAutoTags {
        async fn auto_tag(&self, meeting: &MeetingId) -> Result<AutoTagged, TagError> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(meeting.clone());
            Ok(AutoTagged::Applied(Vec::new()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use deino_core::tags::{AutoTagSkip, TagError};
    use deino_db::DbError;
    use deino_domain::{ErrorCode, FakeClock, TagId};
    use deino_testkit::at;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::RecordedProgress;

    /// Answers once with a scripted result; a second call hangs.
    struct Scripted(Mutex<Option<Result<AutoTagged, TagError>>>);

    #[async_trait]
    impl AutoTagger for Scripted {
        async fn auto_tag(&self, _meeting: &MeetingId) -> Result<AutoTagged, TagError> {
            let next = self.0.lock().unwrap().take();
            match next {
                Some(result) => result,
                None => std::future::pending().await,
            }
        }
    }

    fn stage(result: Option<Result<AutoTagged, TagError>>) -> AutoTagStage {
        let mut ports = StagePorts::fakes();
        ports.auto_tagger = Arc::new(Scripted(Mutex::new(result)));
        AutoTagStage::new(&ports)
    }

    fn ctx(cancel: CancellationToken) -> StageCtx {
        StageCtx {
            attempt: 1,
            cancel,
            clock: Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
            events: Arc::new(RecordedProgress::default()),
        }
    }

    async fn run(result: Result<AutoTagged, TagError>) -> Result<StageOutcome, StageError> {
        stage(Some(result))
            .run(&ctx(CancellationToken::new()), MeetingId::new("meeting-a"))
            .await
    }

    #[tokio::test]
    async fn applied_tags_finish_the_stage_and_skips_carry_their_reason() {
        let tag = TagId::generate(&FakeClock::new(at("2026-10-09T09:00:00Z")));

        let applied = run(Ok(AutoTagged::Applied(vec![tag]))).await;
        let skipped = run(Ok(AutoTagged::Skipped(AutoTagSkip::TaggedByUser))).await;

        assert_eq!(applied, Ok(StageOutcome::Done));
        assert_eq!(
            skipped,
            Ok(StageOutcome::Skipped(AutoTagSkip::TaggedByUser.reason()))
        );
    }

    #[tokio::test]
    async fn storage_failures_retry_and_permanent_ones_wait_for_the_user() {
        let busy = TagError::Storage(DbError::Query(sqlx::Error::PoolTimedOut));
        let tag = TagId::generate(&FakeClock::new(at("2026-10-09T09:00:00Z")));

        let retry = run(Err(busy)).await.unwrap_err();
        let permanent = run(Err(TagError::NotFound(tag))).await.unwrap_err();

        assert!(retry.is_retryable());
        assert!(
            matches!(permanent, StageError::Permanent(error) if error.code == ErrorCode::NotFound)
        );
    }

    #[tokio::test]
    async fn cancelling_stops_a_tagger_that_is_still_waiting_for_the_model() {
        let cancel = CancellationToken::new();
        cancel.cancel();

        let outcome = stage(None)
            .run(&ctx(cancel), MeetingId::new("meeting-a"))
            .await;

        assert_eq!(outcome, Err(StageError::Cancelled));
    }

    #[test]
    fn the_stage_is_auto_tag_at_order_seventy() {
        let stage = stage(None);
        assert_eq!((stage.id(), stage.order()), (StageId::new("auto_tag"), 70));
    }
}
