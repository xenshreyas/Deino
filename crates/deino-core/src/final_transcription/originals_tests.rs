//! The kept-transcript contract on SQLite and the fake, plus what only the
//! schema does: retention and deleting the meeting take the copy along.

use deino_db::test_db;
use deino_domain::{Channel, MeetingId, Millis};
use deino_testkit::{at, seed};

use super::*;

const MEETING: &str = "meeting-a";

fn line(start: u32, text: &str) -> KeptLine {
    KeptLine {
        channel: Channel::Mic,
        start: Millis(start),
        end: Millis(start + 900),
        text: text.into(),
        words: vec![WordTiming {
            text: text.into(),
            start: Millis(start),
            end: Millis(start + 400),
        }],
    }
}

async fn sqlite() -> (Database, SqliteOriginalTranscripts) {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at("2026-10-12T09:00:00Z")).await;
    (database.clone(), SqliteOriginalTranscripts::new(database))
}

/// The first transcript kept stays; forgetting is idempotent.
async fn the_first_kept_transcript_wins(repo: &dyn OriginalTranscripts) {
    let meeting = MeetingId::new(MEETING);
    let at = at("2026-10-12T10:00:00Z");
    assert_eq!(repo.kept(&meeting).await.unwrap(), None);

    repo.keep(&meeting, &[line(0, "local")], at).await.unwrap();
    repo.keep(&meeting, &[line(0, "cloud")], at).await.unwrap();

    assert_eq!(
        repo.kept(&meeting).await.unwrap(),
        Some(vec![line(0, "local")])
    );
    repo.forget(&meeting).await.unwrap();
    repo.forget(&meeting).await.unwrap();
    assert_eq!(repo.kept(&meeting).await.unwrap(), None);
}

#[tokio::test]
async fn sqlite_honours_the_contract() {
    let (_, repo) = sqlite().await;
    the_first_kept_transcript_wins(&repo).await;
}

#[tokio::test]
async fn the_fake_honours_the_contract() {
    the_first_kept_transcript_wins(&FakeOriginalTranscripts::default()).await;
}

#[tokio::test]
async fn retention_and_deleting_the_meeting_delete_the_kept_transcript() {
    let (database, repo) = sqlite().await;
    let meeting = MeetingId::new(MEETING);
    repo.keep(&meeting, &[line(0, "local")], at("2026-10-12T10:00:00Z"))
        .await
        .unwrap();

    sqlx::query("UPDATE meetings SET transcript_deleted_at = '2026-11-12T10:00:00Z' WHERE id = ?1")
        .bind(MEETING)
        .execute(database.writer())
        .await
        .unwrap();
    assert_eq!(repo.kept(&meeting).await.unwrap(), None);

    repo.keep(&meeting, &[line(0, "again")], at("2026-10-12T10:00:00Z"))
        .await
        .unwrap();
    sqlx::query("DELETE FROM meetings WHERE id = ?1")
        .bind(MEETING)
        .execute(database.writer())
        .await
        .unwrap();
    assert_eq!(repo.kept(&meeting).await.unwrap(), None);
}
