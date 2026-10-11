//! `SqliteEnhancedNotesRepo` against the real schema, and the fake against
//! the same contract.

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_doc::{Doc, Flavor};
use deino_domain::{FakeClock, MeetingId, SeriesId, TemplateId};
use deino_testkit::{at, seed};

use super::*;
use crate::templates::{BuiltinSync, CATALOG, SqliteTemplateRepo};
use crate::testing::query_plan;

async fn database() -> Database {
    let database = test_db().await;
    let clock = Arc::new(FakeClock::new(at("2026-10-01T00:00:00Z")));
    let templates = Arc::new(SqliteTemplateRepo::new(database.clone()));
    BuiltinSync::new(templates, clock)
        .run(&CATALOG)
        .await
        .unwrap();
    for (id, title, created) in [
        ("meeting-a", "Maya 1:1", "2026-10-07T15:00:00Z"),
        ("meeting-b", "Standup", "2026-10-08T09:00:00Z"),
        ("meeting-earlier", "Maya 1:1", "2026-09-30T15:00:00Z"),
    ] {
        seed::meeting(&database, id, title, at(created)).await;
    }
    database
}

fn version(meeting: &str, text: &str, at_time: &str) -> NewVersion {
    let doc = deino_doc::from_markdown(text).unwrap();
    NewVersion {
        meeting_id: MeetingId::new(meeting),
        markdown: deino_doc::to_markdown(&doc, Flavor::Readable),
        doc,
        template_id: Some(TemplateId::new("one_on_one")),
        model: "anthropic:claude-sonnet-5-5".into(),
        basis: None,
        is_user_edited: false,
        at: at(at_time),
    }
}

async fn versions_are_numbered_per_meeting(
    repo: &(impl EnhancedNotesReader + EnhancedNotesWriter),
) {
    let a = MeetingId::new("meeting-a");
    assert_eq!(
        repo.insert(&version("meeting-a", "- one", "2026-10-07T16:00:00Z"))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        repo.insert(&version("meeting-a", "- two", "2026-10-07T17:00:00Z"))
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        repo.insert(&version("meeting-b", "- other", "2026-10-08T10:00:00Z"))
            .await
            .unwrap(),
        1
    );

    let latest = repo.latest(&a).await.unwrap().unwrap();
    assert_eq!((latest.version, latest.markdown.as_str()), (2, "- two\n"));
    assert_eq!(
        repo.version(&a, 1).await.unwrap().unwrap().doc,
        deino_doc::from_markdown("- one").unwrap()
    );
    let listed: Vec<u32> = repo
        .versions(&a)
        .await
        .unwrap()
        .iter()
        .map(|v| v.version)
        .collect();
    assert_eq!(listed, [2, 1], "newest first");
    assert_eq!(
        repo.latest(&MeetingId::new("meeting-none")).await.unwrap(),
        None
    );
}

async fn only_the_newest_version_takes_edits(
    repo: &(impl EnhancedNotesReader + EnhancedNotesWriter),
) {
    repo.insert(&version("meeting-a", "- one", "2026-10-07T16:00:00Z"))
        .await
        .unwrap();
    repo.insert(&version("meeting-a", "- two", "2026-10-07T17:00:00Z"))
        .await
        .unwrap();
    let edit = |version: u32| VersionEdit {
        meeting_id: MeetingId::new("meeting-a"),
        version,
        doc: Doc::empty(),
        markdown: String::new(),
        at: at("2026-10-07T18:00:00Z"),
    };

    assert!(!repo.save_edit(&edit(1)).await.unwrap());
    assert!(repo.save_edit(&edit(2)).await.unwrap());

    let latest = repo
        .latest(&MeetingId::new("meeting-a"))
        .await
        .unwrap()
        .unwrap();
    assert!(latest.is_user_edited);
    assert_eq!(latest.updated_at, at("2026-10-07T18:00:00Z"));
    assert!(
        !repo
            .version(&MeetingId::new("meeting-a"), 1)
            .await
            .unwrap()
            .unwrap()
            .is_user_edited
    );
}

#[tokio::test]
async fn sqlite_numbers_versions_per_meeting() {
    versions_are_numbered_per_meeting(&SqliteEnhancedNotesRepo::new(database().await)).await;
}

#[tokio::test]
async fn fake_numbers_versions_per_meeting() {
    versions_are_numbered_per_meeting(&FakeEnhancedNotesRepo::default()).await;
}

#[tokio::test]
async fn sqlite_takes_edits_on_the_newest_version_only() {
    only_the_newest_version_takes_edits(&SqliteEnhancedNotesRepo::new(database().await)).await;
}

#[tokio::test]
async fn fake_takes_edits_on_the_newest_version_only() {
    only_the_newest_version_takes_edits(&FakeEnhancedNotesRepo::default()).await;
}

#[tokio::test]
async fn the_version_list_names_each_template_and_reads_by_primary_key() {
    let database = database().await;
    let repo = SqliteEnhancedNotesRepo::new(database.clone());
    repo.insert(&version("meeting-a", "- one", "2026-10-07T16:00:00Z"))
        .await
        .unwrap();

    let versions = repo.versions(&MeetingId::new("meeting-a")).await.unwrap();
    let plan = query_plan(&database, include_str!("queries/versions.sql")).await;

    assert_eq!(versions[0].template_name.as_deref(), Some("1:1"));
    assert_eq!(
        plan[0],
        "SEARCH e USING INDEX sqlite_autoindex_enhanced_notes_1 (meeting_id=?)"
    );
}

/// Two meetings of one series, each with notes.
async fn series_with_notes(database: &Database) -> SqliteEnhancedNotesRepo {
    sqlx::query("UPDATE meetings SET series_id = 'series-maya' WHERE id IN ('meeting-a', 'meeting-earlier')")
        .execute(database.writer())
        .await
        .unwrap();
    let repo = SqliteEnhancedNotesRepo::new(database.clone());
    repo.insert(&version(
        "meeting-earlier",
        "- beta on track",
        "2026-09-30T16:00:00Z",
    ))
    .await
    .unwrap();
    repo.insert(&version("meeting-a", "- later", "2026-10-07T16:00:00Z"))
        .await
        .unwrap();
    repo
}

#[tokio::test]
async fn the_previous_meeting_of_a_series_brings_its_newest_notes() {
    let database = database().await;
    let repo = series_with_notes(&database).await;
    let series = SeriesId::new("series-maya");

    let previous = repo
        .previous_in_series(
            &series,
            &MeetingId::new("meeting-a"),
            at("2026-10-07T15:00:00Z"),
        )
        .await
        .unwrap()
        .unwrap();
    let none_before = repo
        .previous_in_series(
            &series,
            &MeetingId::new("meeting-earlier"),
            at("2026-09-30T15:00:00Z"),
        )
        .await
        .unwrap();
    let plan = query_plan(&database, include_str!("queries/previous_in_series.sql")).await;

    assert_eq!(
        (previous.title.as_str(), previous.markdown.as_str()),
        ("Maya 1:1", "- beta on track\n")
    );
    assert_eq!(none_before, None);
    assert!(plan[0].contains("idx_meetings_series_id"), "{plan:?}");
}

#[tokio::test]
async fn deleting_the_meeting_deletes_its_notes_and_a_deleted_template_is_forgotten() {
    let database = database().await;
    let repo = SqliteEnhancedNotesRepo::new(database.clone());
    let mut custom = version("meeting-b", "- x", "2026-10-08T10:00:00Z");
    custom.template_id = Some(TemplateId::new("standup"));
    repo.insert(&version("meeting-a", "- one", "2026-10-07T16:00:00Z"))
        .await
        .unwrap();
    repo.insert(&custom).await.unwrap();

    sqlx::query("DELETE FROM meetings WHERE id = 'meeting-a'")
        .execute(database.writer())
        .await
        .unwrap();
    sqlx::query("DELETE FROM templates WHERE id = 'standup'")
        .execute(database.writer())
        .await
        .unwrap();

    assert_eq!(
        repo.latest(&MeetingId::new("meeting-a")).await.unwrap(),
        None
    );
    assert_eq!(
        repo.latest(&MeetingId::new("meeting-b"))
            .await
            .unwrap()
            .unwrap()
            .template_id,
        None
    );
}
