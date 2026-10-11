use deino_db::DbError;
use deino_domain::{FakeClock, MeetingId};
use deino_testkit::at;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::RecordedProgress;
use crate::stages::RecordedFixer;

fn ctx() -> StageCtx {
    StageCtx {
        attempt: 1,
        cancel: CancellationToken::new(),
        clock: Arc::new(FakeClock::new(at("2026-10-11T09:00:00Z"))),
        events: Arc::new(RecordedProgress::default()),
    }
}

/// A fixer that always answers `outcome`, or fails with storage.
struct Answers(Option<FixupOutcome>);

#[async_trait]
impl VocabFixer for Answers {
    async fn fix(&self, _meeting: &MeetingId) -> Result<FixupOutcome, VocabError> {
        self.0.ok_or(VocabError::Storage(DbError::Query(
            sqlx::Error::RowNotFound,
        )))
    }
}

fn stage(fixer: Arc<dyn VocabFixer>) -> VocabFixupsStage {
    VocabFixupsStage::new(&StagePorts {
        vocab_fixer: fixer,
        ..StagePorts::fakes()
    })
}

#[tokio::test]
async fn the_meeting_is_corrected_between_speakers_and_notes() {
    let fixer = Arc::new(RecordedFixer::default());
    let meeting = MeetingId::new("meeting-a");

    let outcome = stage(fixer.clone())
        .run(&ctx(), meeting.clone())
        .await
        .unwrap();

    assert_eq!(outcome, StageOutcome::Done);
    assert_eq!(fixer.meetings(), [meeting]);
    assert_eq!(stage(fixer).order(), 40);
}

#[tokio::test]
async fn an_empty_vocabulary_is_a_skip_and_storage_failures_are_retried() {
    let empty = FixupOutcome {
        empty_vocabulary: true,
        ..FixupOutcome::default()
    };
    let meeting = MeetingId::new("meeting-a");

    let skipped = stage(Arc::new(Answers(Some(empty))))
        .run(&ctx(), meeting.clone())
        .await;
    let failed = stage(Arc::new(Answers(None))).run(&ctx(), meeting).await;

    assert_eq!(skipped, Ok(StageOutcome::Skipped(EMPTY_VOCABULARY)));
    assert!(failed.unwrap_err().is_retryable());
}
