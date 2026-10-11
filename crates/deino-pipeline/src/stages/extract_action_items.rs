//! `extract_action_items` (order 60): the meeting's commitments, with
//! owners and due days, through the LLM gateway's `extract.action_items`
//! route (AI-05).
//!
//! The rules are `deino_core::action_items::extraction` behind the
//! `ActionItemExtractor` port: it replaces only the AI items the user never
//! touched, so a retry or a re-run from the status strip is safe.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::action_items::extraction::{
    ActionItemExtractor, ExtractionError, ExtractionOutcome,
};
use deino_domain::{ErrorClass, MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "extract_action_items";
/// Where it runs: after the notes, before titles and tags (§4.2).
const ORDER: u16 = 60;
/// Shown when there was nothing to read.
const NO_TRANSCRIPT: &str = "The meeting has no transcript to find action items in.";

/// Extracts a meeting's action items.
pub struct ExtractActionItemsStage {
    extractor: Arc<dyn ActionItemExtractor>,
}

impl ExtractActionItemsStage {
    /// The stage over the shared ports.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            extractor: Arc::clone(&ports.action_items),
        }
    }
}

#[async_trait]
impl PipelineStage for ExtractActionItemsStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let outcome = tokio::select! {
            biased;
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
            outcome = self.extractor.extract(&meeting) => outcome,
        };
        match outcome {
            Ok(ExtractionOutcome::NoTranscript) => Ok(StageOutcome::Skipped(NO_TRANSCRIPT)),
            Ok(ExtractionOutcome::Extracted { .. }) => Ok(StageOutcome::Done),
            Err(error) => Err(failure(&error)),
        }
    }
}

/// Rate limits, busy providers and storage errors are retried; a missing
/// key or model waits for the user.
fn failure(error: &ExtractionError) -> StageError {
    if error.is_retryable() {
        StageError::retryable_from(error)
    } else {
        StageError::permanent(error.code(), error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use deino_core::action_items::FakeActionItemExtractor;
    use deino_domain::FakeClock;
    use deino_llm::{LlmError, ProviderId};
    use deino_testkit::at;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::RecordedProgress;

    fn ctx() -> StageCtx {
        StageCtx {
            attempt: 1,
            cancel: CancellationToken::new(),
            clock: Arc::new(FakeClock::new(at("2026-10-08T09:00:00Z"))),
            events: Arc::new(RecordedProgress::default()),
        }
    }

    fn stage(
        extractor: FakeActionItemExtractor,
    ) -> (ExtractActionItemsStage, Arc<FakeActionItemExtractor>) {
        let extractor = Arc::new(extractor);
        let ports = StagePorts {
            action_items: extractor.clone(),
            ..StagePorts::fakes()
        };
        (ExtractActionItemsStage::new(&ports), extractor)
    }

    fn meeting() -> MeetingId {
        MeetingId::new("meeting-a")
    }

    #[tokio::test]
    async fn a_meeting_with_items_is_done_and_was_asked_about() {
        let (stage, extractor) = stage(FakeActionItemExtractor::saving(3));

        assert_eq!(stage.run(&ctx(), meeting()).await, Ok(StageOutcome::Done));
        assert_eq!(extractor.asked(), [meeting()]);
        assert_eq!(
            (stage.id(), stage.order()),
            (StageId::new("extract_action_items"), 60)
        );
    }

    #[tokio::test]
    async fn no_transcript_is_a_skip_with_a_reason() {
        let (stage, _) = stage(FakeActionItemExtractor::answering(Ok(
            ExtractionOutcome::NoTranscript,
        )));

        assert_eq!(
            stage.run(&ctx(), meeting()).await,
            Ok(StageOutcome::Skipped(NO_TRANSCRIPT))
        );
    }

    #[tokio::test]
    async fn a_rate_limit_is_retried_and_a_missing_key_waits_for_the_user() {
        let limited = || {
            ExtractionError::Llm(LlmError::RateLimited {
                provider: ProviderId::Anthropic,
                retry_after: None,
            })
        };
        let no_key = || ExtractionError::Llm(LlmError::NoKey(ProviderId::Anthropic));
        let (retried, _) = stage(FakeActionItemExtractor::answering(Err(limited)));
        let (waiting, _) = stage(FakeActionItemExtractor::answering(Err(no_key)));

        let retried = retried.run(&ctx(), meeting()).await.unwrap_err();
        let waiting = waiting.run(&ctx(), meeting()).await.unwrap_err();

        assert!(retried.is_retryable());
        assert!(!waiting.is_retryable());
        assert!(matches!(waiting, StageError::Permanent(_)));
        assert_ne!(retried, StageError::Cancelled);
    }

    #[tokio::test]
    async fn a_cancelled_pipeline_stops_the_stage() {
        let (stage, _) = stage(FakeActionItemExtractor::saving(1));
        let ctx = ctx();
        ctx.cancel.cancel();

        assert_eq!(stage.run(&ctx, meeting()).await, Err(StageError::Cancelled));
    }
}
