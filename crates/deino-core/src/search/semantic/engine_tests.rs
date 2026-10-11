use deino_domain::MeetingId;

use super::super::test_harness::Harness;
use super::*;

async fn two_meetings(harness: &Harness) -> (MeetingId, MeetingId) {
    let calendar = harness
        .meeting(
            "meeting-cal",
            "Calendar sync",
            &[
                "Microsoft Graph webhook subscriptions expire after three days",
                "We renew them every six hours",
            ],
        )
        .await;
    let hiring = harness
        .meeting(
            "meeting-hire",
            "Hiring",
            &[
                "Jordan expects a salary in euros",
                "Notice period is four weeks",
            ],
        )
        .await;
    harness.indexer.index_meeting(&calendar).await.unwrap();
    harness.indexer.index_meeting(&hiring).await.unwrap();
    (calendar, hiring)
}

#[tokio::test]
async fn the_nearest_chunk_comes_back_as_a_moment_with_its_window_and_best_line() {
    let harness = Harness::new().await;
    let (calendar, _) = two_meetings(&harness).await;

    let hits = harness
        .search()
        .search("graph webhook subscriptions", None, 2)
        .await
        .unwrap()
        .unwrap();

    let best = &hits[0];
    assert_eq!(best.kind, HitKind::Segment);
    assert_eq!(best.meeting_id.as_ref(), Some(&calendar));
    assert_eq!(best.label.text, "Calendar sync");
    assert!(best.ts_ms.is_some() && best.end_ms.is_some());
    assert!(
        best.snippet.text.contains("Microsoft Graph webhook"),
        "{}",
        best.snippet.text
    );
    assert!(!best.snippet.highlights.is_empty());
    assert!(hits.windows(2).all(|pair| pair[0].score >= pair[1].score));
}

#[tokio::test]
async fn a_scope_restricts_the_meetings_searched() {
    let harness = Harness::new().await;
    let (_, hiring) = two_meetings(&harness).await;

    let hits = harness
        .search()
        .search(
            "graph webhook subscriptions",
            Some(std::slice::from_ref(&hiring)),
            5,
        )
        .await
        .unwrap()
        .unwrap();

    assert!(
        hits.iter()
            .all(|hit| hit.meeting_id.as_ref() == Some(&hiring))
    );
}

#[tokio::test]
async fn without_a_model_or_an_index_semantic_search_steps_aside() {
    let harness = Harness::new().await;

    assert_eq!(
        harness.search().search("anything", None, 5).await.unwrap(),
        None
    );
    two_meetings(&harness).await;
    harness.embedder.set_ready(false);
    assert_eq!(
        harness.search().search("graph", None, 5).await.unwrap(),
        None
    );
}

#[tokio::test]
async fn trashed_meetings_drop_out_and_new_meetings_join_a_loaded_index() {
    let harness = Harness::new().await;
    let (calendar, _) = two_meetings(&harness).await;
    let search = harness.search();
    search.search("graph", None, 5).await.unwrap();
    sqlx::query("UPDATE meetings SET deleted_at = '2026-10-10' WHERE id = ?1")
        .bind(&calendar)
        .execute(harness.database.writer())
        .await
        .unwrap();
    let late = harness
        .meeting(
            "meeting-late",
            "Late",
            &["Graph renewal job failed overnight"],
        )
        .await;
    harness.indexer.index_meeting(&late).await.unwrap();

    let hits = search
        .search("graph renewal", None, 5)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(hits[0].meeting_id.as_ref(), Some(&late));
    assert!(
        hits.iter()
            .all(|hit| hit.meeting_id.as_ref() != Some(&calendar))
    );
}
