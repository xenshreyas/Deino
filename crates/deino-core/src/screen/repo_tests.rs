//! What only the real schema can check: the round trip, the order, the
//! de-duplication, the cascade and the index.

use deino_db::test_db;
use deino_testkit::{at, seed};

use super::*;
use crate::testing::query_plan;

const MEETING: &str = "meeting-a";

fn snapshot(at_ms: u32) -> NewSnapshot {
    NewSnapshot {
        meeting: MeetingId::new(MEETING),
        at: Millis(at_ms),
        file_name: format!("{at_ms:010}.png"),
        width: 1_920,
        height: 1_200,
        phash: -42,
    }
}

async fn repo() -> (Database, SqliteScreenRepo) {
    let database = test_db().await;
    seed::meeting(
        &database,
        MEETING,
        "Design review",
        at("2026-10-12T09:00:00Z"),
    )
    .await;
    (database.clone(), SqliteScreenRepo::new(database))
}

#[tokio::test]
async fn snapshots_come_back_in_time_order_once_each() {
    let (_, repo) = repo().await;
    let now = at("2026-10-12T09:30:00Z");

    for at_ms in [20_000, 0, 20_000] {
        repo.add_snapshot(&snapshot(at_ms), now).await.unwrap();
    }

    let rows = repo.snapshots(&MeetingId::new(MEETING)).await.unwrap();
    let times: Vec<Millis> = rows.iter().map(|row| row.at).collect();
    assert_eq!(times, [Millis(0), Millis(20_000)]);
    assert_eq!(rows[1].file_name, "0000020000.png");
    assert_eq!((rows[1].width, rows[1].height), (1_920, 1_200));
}

#[tokio::test]
async fn a_bad_file_name_is_refused_by_the_schema() {
    let (_, repo) = repo().await;
    let mut bad = snapshot(0);
    bad.file_name = "../escape.png".into();

    assert!(
        repo.add_snapshot(&bad, at("2026-10-12T09:30:00Z"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn the_choice_is_kept_per_meeting_and_can_change() {
    let (_, repo) = repo().await;
    let meeting = MeetingId::new(MEETING);
    assert_eq!(repo.capture_choice(&meeting).await.unwrap(), None);

    repo.set_capture(&meeting, false, at("2026-10-12T09:10:00Z"))
        .await
        .unwrap();
    assert_eq!(repo.capture_choice(&meeting).await.unwrap(), Some(false));
    repo.set_capture(&meeting, true, at("2026-10-12T09:20:00Z"))
        .await
        .unwrap();
    assert_eq!(repo.capture_choice(&meeting).await.unwrap(), Some(true));
}

#[tokio::test]
async fn rows_go_with_their_meeting() {
    let (database, repo) = repo().await;
    let meeting = MeetingId::new(MEETING);
    repo.add_snapshot(&snapshot(0), at("2026-10-12T09:30:00Z"))
        .await
        .unwrap();
    repo.set_capture(&meeting, false, at("2026-10-12T09:30:00Z"))
        .await
        .unwrap();

    sqlx::query("DELETE FROM meetings WHERE id = ?1")
        .bind(MEETING)
        .execute(database.writer())
        .await
        .unwrap();

    assert!(repo.snapshots(&meeting).await.unwrap().is_empty());
    assert_eq!(repo.capture_choice(&meeting).await.unwrap(), None);
}

#[tokio::test]
async fn a_meetings_snapshots_are_read_through_the_index() {
    let (database, _) = repo().await;

    let plan = query_plan(
        &database,
        "SELECT id FROM snapshots WHERE meeting_id = ?1 ORDER BY at_ms, id",
    )
    .await;

    assert!(
        plan.iter()
            .any(|step| step.contains("idx_snapshots_meeting_id")),
        "{plan:?}"
    );
}
