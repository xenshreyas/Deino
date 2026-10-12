//! `screen_context` runs before the notes, reads through its port, and
//! skips when there is nothing new or no text recognition.

use deino_core::screen::ScreenContextError;
use deino_db::DbError;
use deino_domain::FakeClock;
use deino_testkit::at;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::RecordedProgress;
use crate::registry::stages;

fn ctx() -> StageCtx {
    StageCtx {
        attempt: 1,
        cancel: CancellationToken::new(),
        clock: Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
        events: Arc::new(RecordedProgress::default()),
    }
}

fn stage(reader: Arc<dyn ScreenContextReader>) -> ScreenContextStage {
    ScreenContextStage::new(&StagePorts {
        screen_context: reader,
        ..StagePorts::fakes()
    })
}

/// A reader that always fails with `error`.
struct Failing(fn() -> ScreenContextError);

#[async_trait]
impl ScreenContextReader for Failing {
    async fn read(&self, _meeting: &MeetingId) -> Result<ScreenContextReport, ScreenContextError> {
        Err((self.0)())
    }
}

#[test]
fn it_runs_after_the_transcript_and_before_the_notes() {
    let ids: Vec<String> = stages(&StagePorts::fakes())
        .iter()
        .map(|stage| stage.id().to_string())
        .collect();
    let at = |id: &str| ids.iter().position(|known| known == id).unwrap();

    assert!(at("vocab_fixups") < at("screen_context"));
    assert!(at("screen_context") < at("enhance_notes"));
}

#[tokio::test]
async fn read_snapshots_are_done_and_nothing_new_is_skipped() {
    let meeting = MeetingId::new("meeting-a");
    let read = Arc::new(RecordedScreenContext::answering(ScreenContextReport {
        read: 2,
        described: 0,
    }));

    let outcome = stage(read.clone()).run(&ctx(), meeting.clone()).await;

    assert_eq!(outcome, Ok(StageOutcome::Done));
    assert_eq!(read.asked(), [meeting.clone()]);
    let nothing = stage(Arc::new(RecordedScreenContext::default()));
    assert_eq!(
        nothing.run(&ctx(), meeting).await,
        Ok(StageOutcome::Skipped(NOTHING_TO_READ))
    );
}

#[tokio::test]
async fn without_ocr_it_skips_and_a_database_failure_retries() {
    let meeting = MeetingId::new("meeting-a");

    let unsupported = stage(Arc::new(Failing(|| ScreenContextError::Unsupported)));
    assert_eq!(
        unsupported.run(&ctx(), meeting.clone()).await,
        Ok(StageOutcome::Skipped(NO_OCR))
    );
    let busy = stage(Arc::new(Failing(|| {
        ScreenContextError::Db(DbError::Query(sqlx::Error::PoolTimedOut))
    })));
    assert!(busy.run(&ctx(), meeting).await.unwrap_err().is_retryable());
}

#[tokio::test]
async fn a_cancelled_run_stops_before_reading() {
    let read = Arc::new(RecordedScreenContext::default());
    let cancelled = ctx();
    cancelled.cancel.cancel();

    let outcome = stage(read.clone()).run(&cancelled, MeetingId::new("meeting-a")).await;

    assert_eq!(outcome, Err(StageError::Cancelled));
    assert!(read.asked().is_empty());
}
