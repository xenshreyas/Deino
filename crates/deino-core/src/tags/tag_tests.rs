//! The tag ports' contract, run against the SQLite repository and the fake,
//! plus what only the real schema can check.

use deino_db::test_db;
use deino_domain::{FakeClock, MeetingId, PaletteColor, TagId, TagSource};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::Meeting;
use crate::testing::query_plan;

const NOW: &str = "2026-10-08T09:00:00Z";

fn tag(name: &str) -> Tag {
    Tag {
        id: TagId::generate(&FakeClock::new(at(NOW))),
        name: name.into(),
        color: PaletteColor::cycle(name.len()),
        description: String::new(),
        auto_apply: false,
        created_at: at(NOW),
        updated_at: at(NOW),
    }
}

fn meetings() -> [Meeting; 2] {
    ["2026-10-01T09:00:00Z", "2026-10-02T09:00:00Z"].map(|created_at| Meeting {
        id: MeetingId::new(format!("meeting-{created_at}")),
        title: "Standup".into(),
        created_at: at(created_at),
        updated_at: at(created_at),
    })
}

fn application(meeting: &Meeting, tag: &Tag, applied_by: TagSource) -> TagApplication {
    TagApplication {
        meeting_id: meeting.id.clone(),
        tag_id: tag.id,
        applied_by,
        applied_at: at(NOW),
    }
}

async fn sqlite_repo() -> SqliteTagRepo {
    let database = test_db().await;
    for meeting in meetings() {
        seed::meeting(
            &database,
            meeting.id.as_str(),
            &meeting.title,
            meeting.created_at,
        )
        .await;
    }
    SqliteTagRepo::new(database)
}

fn names(tags: &[Tag]) -> Vec<&str> {
    tags.iter().map(|tag| tag.name.as_str()).collect()
}

/// Tags list by name, are found by name ignoring case, update and delete.
async fn tags_round_trip(repo: &impl TagRepo) {
    let mut planning = tag("planning");
    repo.create_tag(&tag("Hiring")).await.unwrap();
    repo.create_tag(&planning).await.unwrap();

    assert_eq!(names(&repo.tags().await.unwrap()), ["Hiring", "planning"]);
    assert_eq!(
        repo.tag_by_name("PLANNING").await.unwrap(),
        Some(planning.clone())
    );

    planning.auto_apply = true;
    planning.description = "Roadmap and sprint planning".into();
    assert!(repo.update_tag(&planning).await.unwrap());
    assert_eq!(
        repo.tag(&planning.id).await.unwrap(),
        Some(planning.clone())
    );
    assert!(repo.delete_tag(&planning.id).await.unwrap());
    assert!(!repo.delete_tag(&planning.id).await.unwrap());
}

/// The first applier is kept; tagged meetings list newest first.
async fn tagging_round_trip(repo: &(impl TagRepo + MeetingTagRepo)) {
    let (planning, hiring) = (tag("Planning"), tag("Hiring"));
    repo.create_tag(&planning).await.unwrap();
    repo.create_tag(&hiring).await.unwrap();
    let [older, newer] = meetings();
    for (meeting, tagged, by) in [
        (&older, &planning, TagSource::User),
        (&older, &planning, TagSource::Ai),
        (&newer, &planning, TagSource::Automation),
        (&older, &hiring, TagSource::Ai),
    ] {
        repo.apply_tag(&application(meeting, tagged, by))
            .await
            .unwrap();
    }

    let on_older = repo.tags_of(&older.id).await.unwrap();
    let summary: Vec<(&str, TagSource)> = on_older
        .iter()
        .map(|applied| (applied.tag.name.as_str(), applied.applied_by))
        .collect();
    assert_eq!(
        summary,
        [("Hiring", TagSource::Ai), ("Planning", TagSource::User)]
    );
    assert_eq!(
        repo.meetings_tagged(&planning.id).await.unwrap(),
        [newer, older.clone()]
    );

    assert!(repo.remove_tag(&older.id, &hiring.id).await.unwrap());
    repo.delete_tag(&planning.id).await.unwrap();
    assert!(repo.tags_of(&older.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn sqlite_tags_round_trip() {
    tags_round_trip(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_tags_round_trip() {
    tags_round_trip(&FakeTagRepo::default()).await;
}

#[tokio::test]
async fn sqlite_tagging_round_trip() {
    tagging_round_trip(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_tagging_round_trip() {
    tagging_round_trip(&FakeTagRepo::with_meetings(meetings())).await;
}

#[tokio::test]
async fn tag_names_are_unique_ignoring_case() {
    let repo = sqlite_repo().await;
    repo.create_tag(&tag("Planning")).await.unwrap();

    assert!(repo.create_tag(&tag("planning")).await.is_err());
}

#[tokio::test]
async fn tag_lists_are_read_through_their_indexes() {
    let repo = sqlite_repo().await;
    let plan = |sql| query_plan(&repo.database, sql);

    assert_eq!(
        plan(include_str!("queries/tags.sql")).await,
        ["SCAN tags USING INDEX sqlite_autoindex_tags_2"]
    );
    assert_eq!(
        plan(include_str!("queries/tags_of.sql")).await,
        [
            "SEARCH mt USING INDEX sqlite_autoindex_meeting_tags_1 (meeting_id=?)",
            "SEARCH t USING INDEX sqlite_autoindex_tags_1 (id=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
    assert_eq!(
        plan(include_str!("queries/meetings_tagged.sql")).await,
        [
            "SEARCH mt USING COVERING INDEX idx_meeting_tags_tag_id (tag_id=?)",
            "SEARCH m USING INDEX sqlite_autoindex_meetings_1 (id=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
}
