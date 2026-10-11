//! The notes port's contract, run against SQLite and the fake, plus what
//! only the real database can show: crash survival and the cascade.

use chrono::{DateTime, Utc};
use deino_db::{Database, DbLocation, MIGRATOR, test_db};
use deino_doc::{Block, Doc, Inline};
use deino_domain::{MeetingId, MeetingSource, MeetingStatus};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::{MeetingLifecycleRepo, MeetingTitle, NewMeeting, SqliteMeetingRepo};

const SAVED_AT: &str = "2026-10-08T09:00:00Z";

fn doc(text: &str) -> Doc {
    Doc {
        content: vec![Block::paragraph(vec![Inline::text(text, vec![])])],
    }
}

fn notes(meeting: &str, text: &str, at_time: DateTime<Utc>) -> StoredNotes {
    StoredNotes {
        meeting_id: MeetingId::new(meeting),
        doc: doc(text),
        markdown: format!("{text}\n"),
        created_at: at_time,
        updated_at: at_time,
    }
}

async fn scheduled(database: &Database, id: &str) {
    let meeting = NewMeeting {
        id: MeetingId::new(id),
        title: MeetingTitle::parse("Draft").unwrap(),
        status: MeetingStatus::Scheduled,
        source: MeetingSource::Live,
        created_at: at(SAVED_AT),
        started_at: None,
        folder_path: None,
    };
    SqliteMeetingRepo::new(database.clone())
        .create(&meeting)
        .await
        .unwrap();
}

/// Saves replace the document and keep the first `created_at`.
async fn saves_replace_and_keep_created_at(repo: &impl NotesRepo) {
    let first = notes("meeting-a", "first", at(SAVED_AT));
    let later = notes("meeting-a", "second", at("2026-10-08T09:00:05Z"));

    repo.save(&first).await.unwrap();
    repo.save(&later).await.unwrap();

    let stored = repo
        .get(&MeetingId::new("meeting-a"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.doc, doc("second"));
    assert_eq!(stored.markdown, "second\n");
    assert_eq!(stored.created_at, at(SAVED_AT));
    assert_eq!(stored.updated_at, at("2026-10-08T09:00:05Z"));
    assert_eq!(repo.get(&MeetingId::new("meeting-b")).await.unwrap(), None);
}

async fn sqlite_repo() -> SqliteNotesRepo {
    let database = test_db().await;
    seed::meeting(&database, "meeting-a", "Standup", at(SAVED_AT)).await;
    scheduled(&database, "meeting-draft").await;
    SqliteNotesRepo::new(database)
}

#[tokio::test]
async fn sqlite_saves_replace_and_keep_created_at() {
    saves_replace_and_keep_created_at(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_saves_replace_and_keep_created_at() {
    saves_replace_and_keep_created_at(&FakeNotesRepo::default()).await;
}

#[tokio::test]
async fn notes_survive_the_app_dying_and_reopening_the_database() {
    let folder = tempfile::tempdir().unwrap();
    let location = DbLocation::File(folder.path().join("deino.sqlite"));
    let database = Database::open(&location, &MIGRATOR).await.unwrap();
    seed::meeting(&database, "meeting-live", "Recording", at(SAVED_AT)).await;
    let repo = SqliteNotesRepo::new(database);
    repo.save(&notes(
        "meeting-live",
        "typed during the call",
        at(SAVED_AT),
    ))
    .await
    .unwrap();

    // No clean shutdown: the process is gone and only the files remain.
    std::mem::forget(repo);
    let reopened = SqliteNotesRepo::new(Database::open(&location, &MIGRATOR).await.unwrap());

    let stored = reopened
        .get(&MeetingId::new("meeting-live"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.doc, doc("typed during the call"));
}

#[tokio::test]
async fn rows_written_before_the_notepad_are_read_from_their_markdown() {
    let repo = sqlite_repo().await;
    sqlx::query(
        "INSERT INTO meeting_notes (meeting_id, notes_markdown, created_at, updated_at)
         VALUES ('meeting-a', '# Agenda', ?1, ?1)",
    )
    .bind(at(SAVED_AT))
    .execute(repo.database.writer())
    .await
    .unwrap();

    let stored = repo
        .get(&MeetingId::new("meeting-a"))
        .await
        .unwrap()
        .unwrap();

    assert!(matches!(stored.doc.content[0], Block::Heading { .. }));
}

#[tokio::test]
async fn deleting_a_meeting_deletes_its_notes() {
    let repo = sqlite_repo().await;
    repo.save(&notes("meeting-a", "gone soon", at(SAVED_AT)))
        .await
        .unwrap();

    sqlx::query("DELETE FROM meetings WHERE id = 'meeting-a'")
        .execute(repo.database.writer())
        .await
        .unwrap();

    assert_eq!(repo.get(&MeetingId::new("meeting-a")).await.unwrap(), None);
}
