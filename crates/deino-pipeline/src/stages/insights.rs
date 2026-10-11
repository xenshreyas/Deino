//! `insights` (order 85): the user's custom insights (AI-06), through the
//! LLM gateway's `extract.insights` route.
//!
//! It runs after the notes (50), action items (60), the title (65), tags
//! (70) and folders (75), because insights' conditions read the template,
//! tags and folders those stages settle; and before `automations` (95),
//! whose "generate insight" steps read its answers. The rules are
//! `deino_core::insights::GenerateService` behind the `InsightGenerator`
//! port: the runs the user asked for ("Run on past meetings", the meeting's
//! Insights tab), else every automatic insight that matches. Answers already
//! stored are never asked for again in the same run, so retries are cheap.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::insights::{Attempt, GenerateOutcome, InsightError, InsightGenerator};
use deino_domain::{ErrorClass, MeetingId, StageId};

use crate::{PipelineStage, RetryPolicy, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "insights";
/// Where it runs (§4.2).
const ORDER: u16 = 85;
/// Shown when no insight applies to the meeting.
const NOTHING_TO_RUN: &str = "No insight applies to this meeting.";
/// Shown when insights were asked of a meeting without a transcript.
const NO_TRANSCRIPT: &str = "The meeting has no transcript to read insights from.";

/// Runs a meeting's insights.
pub struct InsightsStage {
    generator: Arc<dyn InsightGenerator>,
}

impl InsightsStage {
    /// The stage over the shared ports.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            generator: Arc::clone(&ports.insights),
        }
    }
}

#[async_trait]
impl PipelineStage for InsightsStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let attempt = if RetryPolicy::DEFAULT.allows_retry_after(ctx.attempt) {
            Attempt::Retryable
        } else {
            Attempt::Last
        };
        let outcome = tokio::select! {
            biased;
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
            outcome = self.generator.generate(&meeting, attempt) => outcome,
        };
        match outcome {
            Ok(GenerateOutcome::NothingToRun) => Ok(StageOutcome::Skipped(NOTHING_TO_RUN)),
            Ok(GenerateOutcome::NoTranscript) => Ok(StageOutcome::Skipped(NO_TRANSCRIPT)),
            Ok(GenerateOutcome::Answered { .. }) => Ok(StageOutcome::Done),
            Err(error) => Err(failure(&error)),
        }
    }
}

/// Rate limits, busy providers and storage errors are retried; a missing
/// key or model waits for the user.
fn failure(error: &InsightError) -> StageError {
    if error.is_retryable() {
        StageError::retryable_from(error)
    } else {
        StageError::permanent(error.code(), error.to_string())
    }
}

/// A generator that records the meetings it ran on and answers as scripted.
#[cfg(any(test, feature = "test-support"))]
#[derive(Default)]
pub struct RecordedInsights {
    ran: std::sync::Mutex<Vec<(MeetingId, Attempt)>>,
    fail_with: std::sync::Mutex<Option<fn() -> InsightError>>,
}

#[cfg(any(test, feature = "test-support"))]
impl RecordedInsights {
    /// Fails every run with the error `make` builds.
    pub fn failing(make: fn() -> InsightError) -> Self {
        Self {
            fail_with: std::sync::Mutex::new(Some(make)),
            ..Self::default()
        }
    }

    /// The meetings run on, with the attempt kind, in order.
    pub fn ran(&self) -> Vec<(MeetingId, Attempt)> {
        self.ran
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[cfg(any(test, feature = "test-support"))]
#[async_trait]
impl InsightGenerator for RecordedInsights {
    async fn generate(
        &self,
        meeting: &MeetingId,
        attempt: Attempt,
    ) -> Result<GenerateOutcome, InsightError> {
        self.ran
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((meeting.clone(), attempt));
        let failure = *self
            .fail_with
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        failure.map_or(Ok(GenerateOutcome::Answered { insights: 1 }), |make| {
            Err(make())
        })
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::FakeClock;
    use deino_llm::{LlmError, ProviderId};
    use deino_testkit::at;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::RecordedProgress;

    fn ctx(attempt: u32) -> StageCtx {
        StageCtx {
            attempt,
            cancel: CancellationToken::new(),
            clock: Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
            events: Arc::new(RecordedProgress::default()),
        }
    }

    fn stage(generator: RecordedInsights) -> (InsightsStage, Arc<RecordedInsights>) {
        let generator = Arc::new(generator);
        let ports = StagePorts {
            insights: generator.clone(),
            ..StagePorts::fakes()
        };
        (InsightsStage::new(&ports), generator)
    }

    fn meeting() -> MeetingId {
        MeetingId::new("meeting-a")
    }

    #[tokio::test]
    async fn the_stage_runs_at_85_and_tells_the_generator_which_attempt_is_last() {
        let (stage, generator) = stage(RecordedInsights::default());

        assert_eq!(stage.run(&ctx(1), meeting()).await, Ok(StageOutcome::Done));
        assert_eq!(stage.run(&ctx(3), meeting()).await, Ok(StageOutcome::Done));
        assert_eq!(
            generator.ran(),
            [(meeting(), Attempt::Retryable), (meeting(), Attempt::Last)]
        );
        assert_eq!((stage.id(), stage.order()), (StageId::new("insights"), 85));
    }

    #[tokio::test]
    async fn a_rate_limit_is_retried_and_a_missing_key_waits_for_the_user() {
        let limited = || {
            InsightError::Llm(LlmError::RateLimited {
                provider: ProviderId::Anthropic,
                retry_after: None,
            })
        };
        let no_key = || InsightError::Llm(LlmError::NoKey(ProviderId::Anthropic));
        let (retried, _) = stage(RecordedInsights::failing(limited));
        let (waiting, _) = stage(RecordedInsights::failing(no_key));

        assert!(
            retried
                .run(&ctx(1), meeting())
                .await
                .unwrap_err()
                .is_retryable()
        );
        assert!(matches!(
            waiting.run(&ctx(1), meeting()).await,
            Err(StageError::Permanent(_))
        ));
    }

    #[tokio::test]
    async fn a_cancelled_pipeline_stops_the_stage() {
        let (stage, _) = stage(RecordedInsights::default());
        let ctx = ctx(1);
        ctx.cancel.cancel();

        assert_eq!(stage.run(&ctx, meeting()).await, Err(StageError::Cancelled));
    }
}
