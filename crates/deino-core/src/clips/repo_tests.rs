//! What only the real `clips` schema can check: ordering, the cascade,
//! the constraints and the indexes the lists use.

use deino_db::test_db;
use deino_domain::{ClipId, FakeClock, MeetingId, Millis};
use deino_testkit::{at, seed};

use super::*;
use crate::testing::query_plan;

fn row(meeting: &str, name: &str, start: u32, at_text: &str) -> ClipRow {
    let at = at(at_text);
    ClipRow {
        id: ClipId::generate(&FakeClock::new(at)),
        meeting_id: MeetingId::new(meeting),
        name: name.into(),
        start: Millis(start),
        end: Millis(start + 1_000),
        media: ClipMedia::Audio,
        path: format!("/r/{meeting}/clips/{name}.m4a"),
        at,
    }
}

async fn repo() -> SqliteClipRepo {
    let database = test_db().await;
    for id in ["meeting-a", "meeting-b"] {
        seed::meeting(&database, id, "Standup", at("2026-10-08T09:00:00Z")).await;
    }
    SqliteClipRepo::new(database)
}

#[tokio::test]
async fn a_meetings_clips_come_in_time_order_and_all_clips_newest_first() {
    let repo = repo().await;
    let late = row("meeting-a", "Late", 9_000, "2026-10-12T09:00:00Z");
    let early = row("meeting-a", "Early", 1_000, "2026-10-12T10:00:00Z");
    let other = row("meeting-b", "Other", 0, "2026-10-12T11:00:00Z");
    for clip in [&late, &early, &other] {
        repo.insert(clip).await.unwrap();
    }

    let names = |clips: Vec<Clip>| clips.into_iter().map(|clip| clip.name).collect::<Vec<_>>();
    assert_eq!(
        names(
            repo.list_for_meeting(&MeetingId::new("meeting-a"))
                .await
                .unwrap()
        ),
        ["Early", "Late"]
    );
    assert_eq!(
        names(repo.list_all().await.unwrap()),
        ["Other", "Early", "Late"]
    );
}

#[tokio::test]
async fn paths_are_unique_and_like_matches_literally() {
    let repo = repo().await;
    let clip = row("meeting-a", "50%_off", 0, "2026-10-12T09:00:00Z");
    repo.insert(&clip).await.unwrap();

    let duplicate = ClipRow {
        id: ClipId::generate(&FakeClock::new(at("2026-10-12T09:01:00Z"))),
        ..clip.clone()
    };
    assert!(repo.insert(&duplicate).await.is_err());
    assert_eq!(
        repo.paths_like("/r/meeting-a/clips/").await.unwrap(),
        [clip.path]
    );
    assert!(repo.paths_like("/r/meeting_a/").await.unwrap().is_empty());
}

#[tokio::test]
async fn an_empty_range_is_refused_by_the_schema() {
    let repo = repo().await;
    let clip = ClipRow {
        end: Millis(0),
        ..row("meeting-a", "Empty", 0, "2026-10-12T09:00:00Z")
    };

    assert!(repo.insert(&clip).await.is_err());
}

#[tokio::test]
async fn the_lists_use_their_indexes() {
    let database = test_db().await;

    let by_meeting = query_plan(
        &database,
        "SELECT id FROM clips WHERE meeting_id = 'm' ORDER BY start_ms, id",
    )
    .await;
    let all = query_plan(
        &database,
        "SELECT id FROM clips ORDER BY created_at DESC, id DESC",
    )
    .await;

    assert!(
        by_meeting
            .iter()
            .any(|step| step.contains("idx_clips_meeting_id_start_ms")),
        "{by_meeting:?}"
    );
    assert!(
        all.iter().any(|step| step.contains("idx_clips_created_at")),
        "{all:?}"
    );
}
