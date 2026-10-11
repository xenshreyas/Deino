//! [`MeetingDigestReader`] on SQLite: summaries from the newest enhanced notes,
//! transcript openings and recent examples, through their indexes.

use deino_db::{Database, test_db};
use deino_domain::{
    Channel, FakeClock, MeetingId, Millis, PaletteColor, SegmentId, TagId, TagSource,
};
use deino_testkit::{at, seed};

use super::*;
use crate::segments::{NewSegment, SegmentWriter, SqliteSegmentRepo};
use crate::testing::query_plan;

const NOW: &str = "2026-10-09T09:00:00Z";

/// Stores `markdown` as version `version` of the meeting's enhanced notes.
async fn notes(database: &Database, meeting: &str, version: u32, markdown: &str) {
    sqlx::query(
        "INSERT INTO enhanced_notes (meeting_id, version, doc_json, markdown, model, created_at, updated_at)
         VALUES (?1, ?2, '{\"type\":\"doc\",\"content\":[]}', ?3, 'builtin:test', ?4, ?4)",
    )
    .bind(meeting)
    .bind(version)
    .bind(markdown)
    .bind(NOW)
    .execute(database.writer())
    .await
    .unwrap();
}

async fn repo_with(meetings: &[(&str, &str)]) -> SqliteTagRepo {
    let database = test_db().await;
    for (id, created_at) in meetings {
        seed::meeting(&database, id, &format!("Title of {id}"), at(created_at)).await;
    }
    SqliteTagRepo::new(database)
}

fn segment(meeting: &str, start_ms: u32, text: &str) -> NewSegment {
    NewSegment {
        id: SegmentId::generate(&FakeClock::new(at(NOW))),
        meeting_id: MeetingId::new(meeting),
        channel: Channel::Mixed,
        speaker_id: None,
        start: Millis(start_ms),
        end: Millis(start_ms + 500),
        text: text.into(),
        words: Vec::new(),
    }
}

#[tokio::test]
async fn a_digest_carries_the_title_and_the_newest_enhanced_notes() {
    let repo = repo_with(&[("done", NOW), ("none", NOW)]).await;
    notes(&repo.database, "done", 1, "## Seats").await;
    notes(&repo.database, "done", 2, "## Seats and pricing").await;

    let done = read(&repo, "done").await.unwrap();
    assert_eq!(
        (done.title.as_str(), done.summary.as_deref()),
        ("Title of done", Some("## Seats and pricing"))
    );
    assert_eq!(read(&repo, "none").await.unwrap().summary, None);
    assert_eq!(read(&repo, "missing").await, None);
}

async fn read(repo: &SqliteTagRepo, meeting: &str) -> Option<MeetingDigest> {
    repo.digest(&MeetingId::new(meeting)).await.unwrap()
}

#[tokio::test]
async fn a_trashed_meeting_has_no_digest() {
    let repo = repo_with(&[("trashed", NOW)]).await;
    sqlx::query("UPDATE meetings SET deleted_at = ?1 WHERE id = 'trashed'")
        .bind(NOW)
        .execute(repo.database.writer())
        .await
        .unwrap();

    assert_eq!(repo.digest(&MeetingId::new("trashed")).await.unwrap(), None);
}

#[tokio::test]
async fn the_transcript_opening_follows_time_order_and_is_cut_to_length() {
    let repo = repo_with(&[("m", NOW)]).await;
    let segments = [
        segment("m", 2_000, "second"),
        segment("m", 1_000, "first"),
        segment("m", 3_000, "third"),
    ];
    SqliteSegmentRepo::new(repo.database.clone())
        .append(&segments)
        .await
        .unwrap();

    let whole = repo
        .transcript_opening(&MeetingId::new("m"), 100)
        .await
        .unwrap();
    let cut = repo
        .transcript_opening(&MeetingId::new("m"), 9)
        .await
        .unwrap();

    assert_eq!(whole, "first second third");
    assert_eq!(cut, "first sec");
}

/// Creates `Customer` and puts it on each of `meetings` by hand.
async fn tag_all(repo: &SqliteTagRepo, meetings: &[&str]) -> Tag {
    let tag = Tag {
        id: TagId::generate(&FakeClock::new(at(NOW))),
        name: "Customer".into(),
        color: PaletteColor::cycle(0),
        description: String::new(),
        auto_apply: true,
        created_at: at(NOW),
        updated_at: at(NOW),
    };
    repo.create_tag(&tag).await.unwrap();
    for meeting in meetings {
        let application = TagApplication {
            meeting_id: MeetingId::new(*meeting),
            tag_id: tag.id,
            applied_by: TagSource::User,
            applied_at: at(NOW),
        };
        repo.apply_tag(&application).await.unwrap();
    }
    tag
}

#[tokio::test]
async fn examples_are_the_newest_tagged_meetings_other_than_the_one_being_tagged() {
    let repo = repo_with(&[
        ("a", "2026-10-01T09:00:00Z"),
        ("b", "2026-10-02T09:00:00Z"),
        ("c", "2026-10-03T09:00:00Z"),
        ("d", "2026-10-04T09:00:00Z"),
    ])
    .await;
    let tag = tag_all(&repo, &["a", "b", "c", "d"]).await;

    let examples = repo
        .examples(&tag.id, &MeetingId::new("c"), 2)
        .await
        .unwrap();

    let ids: Vec<&str> = examples
        .iter()
        .map(|example| example.meeting_id.as_str())
        .collect();
    assert_eq!(ids, ["d", "b"]);
}

#[tokio::test]
async fn digest_queries_read_through_their_indexes() {
    let repo = repo_with(&[]).await;
    let plan = |sql| query_plan(&repo.database, sql);

    assert_eq!(
        plan(include_str!("queries/digest.sql")).await,
        [
            "SEARCH m USING INDEX sqlite_autoindex_meetings_1 (id=?)",
            "CORRELATED SCALAR SUBQUERY 1",
            "SEARCH e USING INDEX sqlite_autoindex_enhanced_notes_1 (meeting_id=?)"
        ]
    );
    assert_eq!(
        plan(include_str!("queries/examples.sql")).await,
        [
            "SEARCH mt USING COVERING INDEX idx_meeting_tags_tag_id (tag_id=?)",
            "SEARCH m USING INDEX sqlite_autoindex_meetings_1 (id=?)",
            "CORRELATED SCALAR SUBQUERY 1",
            "SEARCH e USING INDEX sqlite_autoindex_enhanced_notes_1 (meeting_id=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
    assert_eq!(
        plan(include_str!("queries/links.sql")).await,
        [
            "SCAN mt USING INDEX sqlite_autoindex_meeting_tags_1",
            "SEARCH m USING INDEX sqlite_autoindex_meetings_1 (id=?)"
        ]
    );
}
