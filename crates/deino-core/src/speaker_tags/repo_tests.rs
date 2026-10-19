//! What only the real schema can check: the round trip, the order, the
//! constraints and the index.

use deino_db::test_db;
use deino_testkit::{at, seed};

use super::*;
use crate::testing::query_plan;

const MEETING: &str = "meeting-a";

fn span(start: u32, end: u32, name: &str, app: CallApp) -> TagSpan {
    TagSpan {
        start: Millis(start),
        end: Millis(end),
        name: DisplayName::parse(name).unwrap(),
        app,
    }
}

async fn repo() -> (Database, SqliteSpeakerTagRepo) {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Weekly", at("2026-10-11T09:00:00Z")).await;
    (database.clone(), SqliteSpeakerTagRepo::new(database))
}

#[tokio::test]
async fn appended_spans_come_back_in_time_order_with_their_app() {
    let (_, repo) = repo().await;
    let meeting = MeetingId::new(MEETING);
    let later = [span(4_000, 6_000, "Bea Ruiz", CallApp::Meet)];
    let earlier = [
        span(0, 1_500, "Alex Chen", CallApp::Zoom),
        span(1_500, 4_000, "Cleo", CallApp::Teams),
    ];

    repo.append(&meeting, &later).await.unwrap();
    repo.append(&meeting, &earlier).await.unwrap();
    repo.append(&meeting, &[]).await.unwrap();

    let stored = repo.spans(&meeting).await.unwrap();
    assert_eq!(stored, [&earlier[..], &later[..]].concat());
}

#[tokio::test]
async fn spans_belong_to_their_meeting_and_go_with_it() {
    let (database, repo) = repo().await;
    let meeting = MeetingId::new(MEETING);
    repo.append(&meeting, &[span(0, 900, "Alex", CallApp::Zoom)])
        .await
        .unwrap();

    assert!(
        repo.spans(&MeetingId::new("other"))
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query("DELETE FROM meetings WHERE id = ?1")
        .bind(MEETING)
        .execute(database.writer())
        .await
        .unwrap();
    assert!(repo.spans(&meeting).await.unwrap().is_empty());
}

#[tokio::test]
async fn the_schema_rejects_empty_spans_and_unknown_apps() {
    let (database, repo) = repo().await;
    let meeting = MeetingId::new(MEETING);

    let empty = repo
        .append(&meeting, &[span(500, 500, "Alex", CallApp::Zoom)])
        .await;
    let unknown_app = sqlx::query(
        "INSERT INTO speaker_tag_spans (meeting_id, start_ms, end_ms, display_name, app)
         VALUES (?1, 0, 10, 'Alex', 'webex')",
    )
    .bind(MEETING)
    .execute(database.writer())
    .await;

    assert!(empty.is_err());
    assert!(unknown_app.is_err());
}

#[tokio::test]
async fn a_meetings_spans_are_read_through_the_index() {
    let (database, _) = repo().await;

    let plan = query_plan(
        &database,
        "SELECT start_ms FROM speaker_tag_spans WHERE meeting_id = ?1 ORDER BY start_ms, id",
    )
    .await;

    assert!(
        plan.iter()
            .any(|step| step.contains("idx_speaker_tag_spans_meeting_id")),
        "{plan:?}"
    );
}

#[tokio::test]
async fn the_roster_is_kept_apart_from_the_speaker_spans_and_read_by_its_index() {
    let (database, repo) = repo().await;
    let meeting = MeetingId::new(MEETING);
    let roster = [
        span(0, 60_000, "Bea Ruiz", CallApp::Zoom),
        span(5_000, 9_000, "Cleo", CallApp::Zoom),
    ];

    repo.append_roster(&meeting, &roster[1..]).await.unwrap();
    repo.append_roster(&meeting, &roster[..1]).await.unwrap();
    repo.append_roster(&meeting, &[]).await.unwrap();

    assert_eq!(repo.roster(&meeting).await.unwrap(), roster);
    assert!(repo.spans(&meeting).await.unwrap().is_empty());
    let empty = repo
        .append_roster(&meeting, &[span(500, 500, "Alex", CallApp::Zoom)])
        .await;
    assert!(empty.is_err());
    let plan = query_plan(
        &database,
        "SELECT start_ms FROM speaker_tag_roster WHERE meeting_id = ?1 ORDER BY start_ms, id",
    )
    .await;
    assert!(
        plan.iter()
            .any(|step| step.contains("idx_speaker_tag_roster_meeting_id")),
        "{plan:?}"
    );
}
