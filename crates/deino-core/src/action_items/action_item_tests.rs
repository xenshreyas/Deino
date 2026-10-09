//! The action item port's contract, run against the SQLite repository and
//! the fake, plus what only the real schema can check.

use chrono::{DateTime, TimeDelta, Utc};
use deino_db::test_db;
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemPriority, ActionItemStatus, FakeClock, MeetingId,
};
use deino_testkit::{at, seed};

use super::*;
use crate::testing::query_plan;

const NOW: &str = "2026-10-08T09:00:00Z";
const MEETING: &str = "meeting-a";

fn item(title: &str, due_in_days: Option<i64>) -> ActionItem {
    ActionItem {
        id: ActionItemId::generate(&FakeClock::new(at(NOW))),
        meeting_id: Some(MeetingId::new(MEETING)),
        title: title.into(),
        description: String::new(),
        assignee_person_id: None,
        due_at: due_in_days.map(|days| at(NOW) + TimeDelta::days(days)),
        status: ActionItemStatus::Open,
        priority: None,
        source_segment_id: None,
        completed_at: None,
        created_by: ActionItemOrigin::Ai,
        created_at: at(NOW),
        updated_at: at(NOW),
    }
}

async fn sqlite_repo() -> SqliteActionItemRepo {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at(NOW)).await;
    SqliteActionItemRepo::new(database)
}

fn titles(items: &[ActionItem]) -> Vec<&str> {
    items.iter().map(|item| item.title.as_str()).collect()
}

/// Open items list by due date with undated ones last; done items leave it.
async fn the_open_list_orders_by_due_date(repo: &impl ActionItemRepo) {
    let done = item("done", Some(0));
    for created in [
        &item("undated", None),
        &item("later", Some(5)),
        &done,
        &item("soon", Some(1)),
    ] {
        repo.create(created).await.unwrap();
    }

    assert!(
        repo.set_status(&done.id, ActionItemStatus::Done, at(NOW))
            .await
            .unwrap()
    );

    assert_eq!(
        titles(&repo.list_open().await.unwrap()),
        ["soon", "later", "undated"]
    );
    let for_meeting = repo
        .list_for_meeting(&MeetingId::new(MEETING))
        .await
        .unwrap();
    assert_eq!(for_meeting.len(), 4);
}

/// Completing stamps `completed_at` once; reopening clears it.
async fn completion_follows_the_status(repo: &impl ActionItemRepo) {
    let task = item("send the deck", None);
    repo.create(&task).await.unwrap();
    let first: DateTime<Utc> = at("2026-10-09T09:00:00Z");

    repo.set_status(&task.id, ActionItemStatus::Done, first)
        .await
        .unwrap();
    repo.set_status(
        &task.id,
        ActionItemStatus::Done,
        first + TimeDelta::hours(1),
    )
    .await
    .unwrap();
    let done = repo.find(&task.id).await.unwrap().unwrap();
    assert_eq!(
        (done.status, done.completed_at),
        (ActionItemStatus::Done, Some(first))
    );

    repo.set_status(
        &task.id,
        ActionItemStatus::Open,
        first + TimeDelta::hours(2),
    )
    .await
    .unwrap();
    let reopened = repo.find(&task.id).await.unwrap().unwrap();
    assert_eq!(
        (reopened.status, reopened.completed_at),
        (ActionItemStatus::Open, None)
    );
    assert_eq!(reopened.updated_at, first + TimeDelta::hours(2));
}

/// Updates save the editable fields; deletes report what they removed.
async fn items_update_and_delete(repo: &impl ActionItemRepo) {
    let mut task = item("draft", None);
    repo.create(&task).await.unwrap();

    task.title = "Send the deck".into();
    task.priority = Some(ActionItemPriority::High);
    task.updated_at = at("2026-10-09T09:00:00Z");
    assert!(repo.update(&task).await.unwrap());
    assert_eq!(repo.find(&task.id).await.unwrap(), Some(task.clone()));

    assert!(repo.delete(&task.id).await.unwrap());
    assert!(!repo.delete(&task.id).await.unwrap());
    assert!(!repo.update(&task).await.unwrap());
}

#[tokio::test]
async fn sqlite_the_open_list_orders_by_due_date() {
    the_open_list_orders_by_due_date(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_the_open_list_orders_by_due_date() {
    the_open_list_orders_by_due_date(&FakeActionItemRepo::default()).await;
}

#[tokio::test]
async fn sqlite_completion_follows_the_status() {
    completion_follows_the_status(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_completion_follows_the_status() {
    completion_follows_the_status(&FakeActionItemRepo::default()).await;
}

#[tokio::test]
async fn sqlite_items_update_and_delete() {
    items_update_and_delete(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_items_update_and_delete() {
    items_update_and_delete(&FakeActionItemRepo::default()).await;
}

#[tokio::test]
async fn the_schema_keeps_completed_at_and_status_in_step() {
    let repo = sqlite_repo().await;
    let mut inconsistent = item("done without a date", None);
    inconsistent.status = ActionItemStatus::Done;

    assert!(repo.create(&inconsistent).await.is_err());
}

#[tokio::test]
async fn every_priority_and_origin_round_trips_through_the_schema() {
    let repo = sqlite_repo().await;
    for (index, priority) in ActionItemPriority::ALL.iter().enumerate() {
        let mut task = item(priority.as_str(), Some(i64::try_from(index).unwrap()));
        task.priority = Some(*priority);
        task.created_by = ActionItemOrigin::ALL[index % ActionItemOrigin::ALL.len()];
        repo.create(&task).await.unwrap();

        assert_eq!(repo.find(&task.id).await.unwrap(), Some(task));
    }
}

#[tokio::test]
async fn deleting_the_meeting_keeps_its_items() {
    let repo = sqlite_repo().await;
    let task = item("follow up", None);
    repo.create(&task).await.unwrap();

    sqlx::query("DELETE FROM meetings WHERE id = ?1")
        .bind(MEETING)
        .execute(repo.database.writer())
        .await
        .unwrap();

    let kept = repo.find(&task.id).await.unwrap().unwrap();
    assert_eq!(kept.meeting_id, None);
}

#[tokio::test]
async fn action_item_lists_are_read_through_their_indexes() {
    let repo = sqlite_repo().await;
    let plan = |sql| query_plan(&repo.database, sql);

    assert_eq!(
        plan(include_str!("queries/open.sql")).await,
        [
            "SEARCH action_items USING INDEX idx_action_items_status_due_at (status=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
    assert_eq!(
        plan(include_str!("queries/for_meeting.sql")).await,
        ["SEARCH action_items USING INDEX idx_action_items_meeting_id (meeting_id=?)"]
    );
    assert_eq!(
        plan(include_str!("queries/for_assignee.sql")).await,
        [
            "SEARCH action_items USING INDEX idx_action_items_assignee_status_due_at (assignee_person_id=? AND status=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
}
