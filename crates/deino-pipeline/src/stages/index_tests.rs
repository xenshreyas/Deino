use std::sync::Mutex;

use deino_core::search::SearchError;
use deino_core::search::semantic::IndexedMeeting;
use deino_db::DbError;
use deino_domain::{ErrorCode, FakeClock};
use deino_testkit::at;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::RecordedProgress;

/// Answers once with a scripted result; a second call hangs.
struct Scripted(Mutex<Option<Result<MeetingIndexOutcome, SearchError>>>);

#[async_trait]
impl SearchIndexing for Scripted {
    async fn index_meeting(
        &self,
        _meeting: &MeetingId,
    ) -> Result<MeetingIndexOutcome, SearchError> {
        let next = self.0.lock().unwrap().take();
        match next {
            Some(result) => result,
            None => std::future::pending().await,
        }
    }
}

fn stage(result: Option<Result<MeetingIndexOutcome, SearchError>>) -> IndexStage {
    let mut ports = StagePorts::fakes();
    ports.search = Arc::new(Scripted(Mutex::new(result)));
    IndexStage::new(&ports)
}

fn ctx(cancel: CancellationToken) -> StageCtx {
    StageCtx {
        attempt: 1,
        cancel,
        clock: Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
        events: Arc::new(RecordedProgress::default()),
    }
}

async fn run(result: Result<MeetingIndexOutcome, SearchError>) -> Result<StageOutcome, StageError> {
    stage(Some(result))
        .run(&ctx(CancellationToken::new()), MeetingId::new("meeting-a"))
        .await
}

#[tokio::test]
async fn an_indexed_meeting_is_done_and_a_missing_model_skips_with_a_reason() {
    let indexed = run(Ok(MeetingIndexOutcome::Indexed(IndexedMeeting::default()))).await;
    let keyword_only = run(Ok(MeetingIndexOutcome::KeywordOnly)).await;

    assert_eq!(indexed, Ok(StageOutcome::Done));
    assert_eq!(keyword_only, Ok(StageOutcome::Skipped(NO_MODEL)));
}

#[tokio::test]
async fn a_busy_database_or_sidecar_retries_and_a_bad_model_file_waits() {
    let busy = SearchError::Storage(DbError::Query(sqlx::Error::PoolTimedOut));
    let sidecar = SearchError::Embedding("the built-in model stopped unexpectedly".into());
    let model = SearchError::Model(deino_core::models::ModelError::HashMismatch);

    assert!(run(Err(busy)).await.unwrap_err().is_retryable());
    assert!(run(Err(sidecar)).await.unwrap_err().is_retryable());
    let permanent = run(Err(model)).await.unwrap_err();
    assert!(matches!(permanent, StageError::Permanent(error) if error.code == ErrorCode::Network));
}

#[tokio::test]
async fn cancelling_stops_an_index_that_is_still_embedding() {
    let cancel = CancellationToken::new();
    cancel.cancel();

    let outcome = stage(None)
        .run(&ctx(cancel), MeetingId::new("meeting-a"))
        .await;

    assert_eq!(outcome, Err(StageError::Cancelled));
}

#[test]
fn the_stage_is_index_at_order_eighty() {
    let stage = stage(None);
    assert_eq!((stage.id(), stage.order()), (StageId::new("index"), 80));
}
