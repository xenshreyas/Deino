//! The NOT-02 stages: their place, the automatic-notes switch, and how
//! enhance failures become retries or user-safe permanent failures.

use std::sync::{Arc, Mutex};

use deino_core::enhance::{EnhanceError, PipelineRun, UP_TO_DATE};
use deino_domain::{AppError, ErrorCode, FakeClock, MeetingId};
use deino_llm::{LlmError, ProviderId};
use deino_testkit::at;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::RecordedProgress;
use crate::stages::{AutoTitleStage, RecordedEnhancer, RecordedTitler};

fn ctx() -> StageCtx {
    StageCtx {
        attempt: 1,
        cancel: CancellationToken::new(),
        clock: Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
        events: Arc::new(RecordedProgress::default()),
    }
}

/// Answers once with a scripted result.
struct Scripted(Mutex<Option<Result<PipelineRun, EnhanceError>>>);

#[async_trait]
impl NotesEnhancer for Scripted {
    async fn enhance(&self, _meeting: &MeetingId) -> Result<PipelineRun, EnhanceError> {
        self.0.lock().unwrap().take().unwrap()
    }
}

fn stage_with(result: Result<PipelineRun, EnhanceError>) -> EnhanceNotesStage {
    let ports = StagePorts {
        enhancer: Arc::new(Scripted(Mutex::new(Some(result)))),
        ..StagePorts::fakes()
    };
    EnhanceNotesStage::new(&ports)
}

#[test]
fn the_notes_stages_run_at_fifty_and_sixty_five() {
    let ports = StagePorts::fakes();

    let notes = EnhanceNotesStage::new(&ports);
    let title = AutoTitleStage::new(&ports);

    assert_eq!(
        (notes.id(), notes.order()),
        (StageId::new("enhance_notes"), 50)
    );
    assert_eq!(
        (title.id(), title.order()),
        (StageId::new("auto_title"), 65)
    );
}

#[tokio::test]
async fn automatic_notes_off_skips_both_stages_without_calling_the_llm() {
    let enhancer = Arc::new(RecordedEnhancer::default());
    let titler = Arc::new(RecordedTitler::default());
    let ports = StagePorts {
        enhancer: enhancer.clone(),
        titler: titler.clone(),
        automatic: Arc::new(crate::stages::FixedAutomaticNotes(false)),
        ..StagePorts::fakes()
    };
    let meeting = MeetingId::new("meeting-a");

    let notes = EnhanceNotesStage::new(&ports)
        .run(&ctx(), meeting.clone())
        .await;
    let title = AutoTitleStage::new(&ports).run(&ctx(), meeting).await;

    assert_eq!(notes, Ok(StageOutcome::Skipped(AUTOMATIC_NOTES_OFF)));
    assert_eq!(title, Ok(StageOutcome::Skipped(AUTOMATIC_NOTES_OFF)));
    assert!(enhancer.meetings().is_empty() && titler.meetings().is_empty());
}

#[tokio::test]
async fn a_written_version_is_done_and_a_skip_keeps_its_reason() {
    let written = stage_with(Ok(PipelineRun::Written(2)))
        .run(&ctx(), MeetingId::new("m"))
        .await;
    let skipped = stage_with(Ok(PipelineRun::Skipped(UP_TO_DATE)))
        .run(&ctx(), MeetingId::new("m"))
        .await;

    assert_eq!(written, Ok(StageOutcome::Done));
    assert_eq!(skipped, Ok(StageOutcome::Skipped(UP_TO_DATE)));
}

#[tokio::test]
async fn a_missing_model_fails_for_good_with_what_to_do() {
    let unavailable = EnhanceError::Llm(LlmError::Unavailable {
        provider: ProviderId::BuiltIn,
        reason: "no model".into(),
    });

    let outcome = stage_with(Err(unavailable))
        .run(&ctx(), MeetingId::new("m"))
        .await;

    assert_eq!(
        outcome,
        Err(StageError::Permanent(AppError::new(
            ErrorCode::Unavailable,
            "Add an Anthropic key or pick a local model in Settings → AI."
        )))
    );
}

#[tokio::test]
async fn a_rate_limit_is_retried_and_a_cancelled_call_cancels_the_stage() {
    let limited = EnhanceError::Llm(LlmError::RateLimited {
        provider: ProviderId::Anthropic,
        retry_after: None,
    });

    let retried = stage_with(Err(limited))
        .run(&ctx(), MeetingId::new("m"))
        .await;
    let cancelled = stage_with(Err(EnhanceError::Llm(LlmError::Cancelled)))
        .run(&ctx(), MeetingId::new("m"))
        .await;

    assert!(retried.unwrap_err().is_retryable());
    assert_eq!(cancelled, Err(StageError::Cancelled));
}

#[tokio::test]
async fn cancelling_the_pipeline_stops_the_stage() {
    let ctx = ctx();
    ctx.cancel.cancel();

    let outcome = EnhanceNotesStage::new(&StagePorts::fakes())
        .run(&ctx, MeetingId::new("m"))
        .await;

    assert_eq!(outcome, Err(StageError::Cancelled));
}
