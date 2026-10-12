//! PRV-01: once retention deleted a meeting's transcript, its notes stay
//! but are never written again, by hand or by the pipeline.

use deino_domain::{ErrorClass, ErrorCode};

use super::*;

/// Deletes the fixture's transcript the way the retention sweep does.
async fn delete_transcript(harness: &Harness) {
    let id = harness.fixture.id.as_str();
    sqlx::query("DELETE FROM transcript_segments WHERE meeting_id = ?1")
        .bind(id)
        .execute(harness.database.writer())
        .await
        .unwrap();
    sqlx::query("UPDATE meetings SET transcript_deleted_at = '2026-10-09T09:00:00Z' WHERE id = ?1")
        .bind(id)
        .execute(harness.database.writer())
        .await
        .unwrap();
}

#[tokio::test]
async fn regenerating_is_refused_once_the_transcript_is_deleted() {
    let harness = harness(MeetingKind::OneOnOne, MockProvider::new()).await;
    delete_transcript(&harness).await;

    let error = harness
        .service
        .enhance(&harness.fixture.id, Overwrite::ReplaceEdits)
        .await
        .unwrap_err();

    assert!(
        matches!(error, EnhanceError::TranscriptDeleted),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::Invalid);
    assert!(enhance_requests(&harness.mock).is_empty(), "no LLM call");
}

#[tokio::test]
async fn the_pipeline_keeps_the_notes_once_the_transcript_is_deleted() {
    let harness = harness(MeetingKind::OneOnOne, MockProvider::new()).await;
    delete_transcript(&harness).await;

    let run = harness
        .service
        .run_for_pipeline(&harness.fixture.id)
        .await
        .unwrap();

    assert_eq!(run, PipelineRun::Skipped(TRANSCRIPT_DELETED));
    assert!(enhance_requests(&harness.mock).is_empty());
}
