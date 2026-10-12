//! `follow_up_draft` (order 98, NOT-07): a suggested follow-up email for
//! meetings with an external attendee and at most ten people, through the
//! LLM gateway's `followup.email` route.
//!
//! It runs after the notes (50), action items (60), the title (65) and
//! people and companies (90), whose output it reads, and before
//! `auto_export` (99). The rules are `deino_core::followups::FollowUpGenerator`
//! behind the [`FollowUpDrafter`] port; this stage maps its outcome and
//! errors.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::followups::{FollowUpDrafter, FollowUpError, FollowUpRun, Kept};
use deino_domain::{MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "follow_up_draft";
/// Where it runs (PARITY_PLAN §4.2).
const ORDER: u16 = 98;
/// Shown when the user dismissed the follow-up.
const DISMISSED: &str = "You dismissed this follow-up.";
/// Shown when the user edited the draft.
const EDITED: &str = "You edited the draft; it was kept.";
/// Shown when there is no transcript or notes.
const NOTHING: &str = "The meeting has no transcript or notes.";

/// Drafts the meeting's follow-up email.
pub struct FollowUpDraftStage {
    drafter: Arc<dyn FollowUpDrafter>,
}

impl FollowUpDraftStage {
    /// The stage over the shared ports.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            drafter: Arc::clone(&ports.follow_ups),
        }
    }
}

crate::registry::register_stage!(FollowUpDraftStage);

#[async_trait]
impl PipelineStage for FollowUpDraftStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let run = tokio::select! {
            biased;
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
            run = self.drafter.draft(&meeting) => run,
        };
        match run {
            Ok(run) => Ok(outcome(run)),
            Err(error) => Err(failure(error)),
        }
    }
}

fn outcome(run: FollowUpRun) -> StageOutcome {
    match run {
        FollowUpRun::Drafted => StageOutcome::Done,
        FollowUpRun::Quiet(reason) => StageOutcome::Skipped(reason.reason()),
        FollowUpRun::Ineligible(why) => StageOutcome::Skipped(why.reason()),
        FollowUpRun::Kept(Kept::Dismissed) => StageOutcome::Skipped(DISMISSED),
        FollowUpRun::Kept(Kept::Edited) => StageOutcome::Skipped(EDITED),
        FollowUpRun::NothingToRead => StageOutcome::Skipped(NOTHING),
    }
}

/// Rate limits, busy providers and storage errors are retried; a missing
/// key or model waits for the user.
fn failure(error: FollowUpError) -> StageError {
    if error.is_retryable() {
        StageError::retryable_from(&error)
    } else {
        StageError::Permanent(error.into())
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedFollowUps;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::{Mutex, PoisonError};

    use async_trait::async_trait;
    use deino_core::followups::{FollowUpDrafter, FollowUpError, FollowUpRun, Ineligible};
    use deino_domain::MeetingId;

    /// A [`FollowUpDrafter`] for tests: records each meeting and drafts
    /// nothing.
    #[derive(Debug, Default)]
    pub struct RecordedFollowUps {
        meetings: Mutex<Vec<MeetingId>>,
    }

    impl RecordedFollowUps {
        /// Every meeting asked about, oldest first.
        pub fn meetings(&self) -> Vec<MeetingId> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl FollowUpDrafter for RecordedFollowUps {
        async fn draft(&self, meeting: &MeetingId) -> Result<FollowUpRun, FollowUpError> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(meeting.clone());
            Ok(FollowUpRun::Ineligible(Ineligible::Internal))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use deino_core::followups::{Ineligible, QuietReason};
    use deino_db::DbError;
    use deino_domain::{ErrorCode, FakeClock};
    use deino_llm::{LlmError, ProviderId};
    use deino_testkit::at;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::RecordedProgress;

    /// Answers once with a scripted result; a second call hangs.
    struct Scripted(Mutex<Option<Result<FollowUpRun, FollowUpError>>>);

    #[async_trait]
    impl FollowUpDrafter for Scripted {
        async fn draft(&self, _meeting: &MeetingId) -> Result<FollowUpRun, FollowUpError> {
            let next = self.0.lock().unwrap().take();
            match next {
                Some(result) => result,
                None => std::future::pending().await,
            }
        }
    }

    fn stage(result: Option<Result<FollowUpRun, FollowUpError>>) -> FollowUpDraftStage {
        let mut ports = StagePorts::fakes();
        ports.follow_ups = Arc::new(Scripted(Mutex::new(result)));
        FollowUpDraftStage::new(&ports)
    }

    fn ctx(cancel: CancellationToken) -> StageCtx {
        StageCtx {
            attempt: 1,
            cancel,
            clock: Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
            events: Arc::new(RecordedProgress::default()),
        }
    }

    async fn run(result: Result<FollowUpRun, FollowUpError>) -> Result<StageOutcome, StageError> {
        stage(Some(result))
            .run(&ctx(CancellationToken::new()), MeetingId::new("meeting-a"))
            .await
    }

    #[tokio::test]
    async fn a_draft_finishes_the_stage_and_every_skip_says_why() {
        assert_eq!(run(Ok(FollowUpRun::Drafted)).await, Ok(StageOutcome::Done));
        assert_eq!(
            run(Ok(FollowUpRun::Ineligible(Ineligible::Internal))).await,
            Ok(StageOutcome::Skipped(Ineligible::Internal.reason()))
        );
        assert_eq!(
            run(Ok(FollowUpRun::Quiet(QuietReason::MovedToChat))).await,
            Ok(StageOutcome::Skipped(QuietReason::MovedToChat.reason()))
        );
        assert_eq!(
            run(Ok(FollowUpRun::Kept(Kept::Dismissed))).await,
            Ok(StageOutcome::Skipped(DISMISSED))
        );
    }

    #[tokio::test]
    async fn a_busy_provider_retries_and_a_missing_model_waits_for_the_user() {
        let busy = FollowUpError::Storage(DbError::Query(sqlx::Error::PoolTimedOut));
        let missing = FollowUpError::Llm(LlmError::Unavailable {
            provider: ProviderId::BuiltIn,
            reason: "no model".into(),
        });

        let retry = run(Err(busy)).await.unwrap_err();
        let permanent = run(Err(missing)).await.unwrap_err();

        assert!(retry.is_retryable());
        assert!(
            matches!(permanent, StageError::Permanent(error) if error.code == ErrorCode::Unavailable)
        );
    }

    #[tokio::test]
    async fn cancelling_stops_a_draft_that_is_still_waiting_for_the_model() {
        let cancel = CancellationToken::new();
        cancel.cancel();

        let outcome = stage(None)
            .run(&ctx(cancel), MeetingId::new("meeting-a"))
            .await;

        assert_eq!(outcome, Err(StageError::Cancelled));
    }

    #[test]
    fn the_stage_is_follow_up_draft_at_order_ninety_eight() {
        let stage = stage(None);
        assert_eq!(
            (stage.id(), stage.order()),
            (StageId::new("follow_up_draft"), 98)
        );
    }
}
