//! [`MeetingTrashRepo`] against the real schema.

use deino_db::test_db;
use deino_domain::MeetingId;
use deino_testkit::{at, seed};

use super::*;
use crate::testing::query_plan;

async fn repo_with(ids: &[&str]) -> SqliteMeetingRepo {
    let database = test_db().await;
    for (day, id) in ids.iter().enumerate() {
        let created_at =
            at("2026-10-01T09:00:00Z") + chrono::TimeDelta::days(i64::try_from(day).unwrap());
        seed::meeting(&database, id, "Standup", created_at).await;
    }
    SqliteMeetingRepo::new(database)
}

fn ids(meetings: &[Meeting]) -> Vec<&str> {
    meetings.iter().map(|meeting| meeting.id.as_str()).collect()
}

#[tokio::test]
async fn trashed_meetings_leave_the_list_and_come_back_on_restore() {
    let repo = repo_with(&["meeting-a", "meeting-b"]).await;
    let id = MeetingId::new("meeting-a");

    assert!(repo.trash(&id, at("2026-10-05T09:00:00Z")).await.unwrap());
    assert_eq!(ids(&repo.list().await.unwrap()), ["meeting-b"]);
    assert_eq!(ids(&repo.list_trashed().await.unwrap()), ["meeting-a"]);

    assert!(repo.restore(&id, at("2026-10-06T09:00:00Z")).await.unwrap());
    assert_eq!(ids(&repo.list().await.unwrap()), ["meeting-b", "meeting-a"]);
    assert!(repo.list_trashed().await.unwrap().is_empty());
}

#[tokio::test]
async fn trashing_twice_or_restoring_a_live_meeting_changes_nothing() {
    let repo = repo_with(&["meeting-a"]).await;
    let id = MeetingId::new("meeting-a");

    assert!(!repo.restore(&id, at("2026-10-05T09:00:00Z")).await.unwrap());
    assert!(repo.trash(&id, at("2026-10-05T09:00:00Z")).await.unwrap());
    assert!(!repo.trash(&id, at("2026-10-06T09:00:00Z")).await.unwrap());

    let state = repo.state(&id).await.unwrap().unwrap();
    assert_eq!(state.deleted_at, Some(at("2026-10-05T09:00:00Z")));
}

#[tokio::test]
async fn the_trash_lists_the_most_recently_trashed_first_through_its_index() {
    let repo = repo_with(&["meeting-a", "meeting-b"]).await;
    repo.trash(&MeetingId::new("meeting-b"), at("2026-10-05T09:00:00Z"))
        .await
        .unwrap();
    repo.trash(&MeetingId::new("meeting-a"), at("2026-10-06T09:00:00Z"))
        .await
        .unwrap();

    let trashed = repo.list_trashed().await.unwrap();
    let plan = query_plan(&repo.database, include_str!("queries/list_trashed.sql")).await;

    assert_eq!(ids(&trashed), ["meeting-a", "meeting-b"]);
    assert_eq!(
        plan,
        ["SEARCH meetings USING INDEX idx_meetings_trashed_deleted_at (deleted_at>?)"]
    );
}

#[tokio::test]
async fn deleting_a_meeting_removes_it_and_its_rows_for_good() {
    let repo = repo_with(&["meeting-a"]).await;
    seed::notes(&repo.database, "meeting-a", "- ship it").await;
    let id = MeetingId::new("meeting-a");

    assert!(repo.delete(&id).await.unwrap());
    assert!(!repo.delete(&id).await.unwrap());

    let notes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM meeting_notes")
        .fetch_one(repo.database.reader())
        .await
        .unwrap();
    assert_eq!(notes, 0, "foreign keys cascade to the meeting's rows");
    assert_eq!(repo.find(&id).await.unwrap(), None);
}
