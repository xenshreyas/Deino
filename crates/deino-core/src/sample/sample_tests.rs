//! The sample meeting: the bundled resource is consistent, and the importer
//! writes it once through the real SQLite repositories.

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::{Database, DbError, test_db};
use deino_domain::{Clock, FakeClock, MeetingId, MeetingStatus};
use deino_testkit::at;

use super::*;
use crate::action_items::SqliteActionItemRepo;
use crate::enhance::{
    EnhancedNotesReader, EnhancedNotesWriter, NewVersion, SqliteEnhancedNotesRepo, VersionEdit,
};
use crate::meetings::{MeetingLifecycleRepo, MeetingTrashRepo, SqliteMeetingRepo};
use crate::segments::{SegmentReader, SpeakerRepo, SqliteSegmentRepo};

fn ports(database: &Database) -> SamplePorts {
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-11T09:00:00Z")));
    SamplePorts {
        meetings: meetings.clone(),
        trash: meetings,
        speakers: segments.clone(),
        segments,
        notes: Arc::new(SqliteEnhancedNotesRepo::new(database.clone())),
        action_items: Arc::new(SqliteActionItemRepo::new(database.clone())),
        clock,
    }
}

async fn count(database: &Database, sql: &str) -> i64 {
    sqlx::query_scalar(sql)
        .bind(SAMPLE_MEETING_ID)
        .fetch_one(database.reader())
        .await
        .unwrap()
}

#[test]
fn the_bundled_sample_parses_and_is_consistent() {
    let sample = SampleMeeting::bundled().unwrap();
    assert!(sample.title.starts_with("Sample"));
    assert_eq!(
        sample
            .speakers
            .iter()
            .filter(|speaker| speaker.is_me)
            .count(),
        1
    );
    assert!(sample.segments.len() >= 20);
    assert_eq!(sample.action_items.len(), 3);
}

#[test]
fn parse_rejects_an_unknown_speaker_and_an_item_past_the_transcript() {
    let speaker = r#"{"title":"Sample","durationMs":1,"speakers":[],"notes":"x","actionItems":[],
        "segments":[{"speaker":"Ghost","channel":"mic","startMs":0,"endMs":1,"text":"hi"}]}"#;
    assert!(matches!(
        SampleMeeting::parse(speaker),
        Err(SampleError::Invalid(_))
    ));
    let item = r#"{"title":"Sample","durationMs":1,"speakers":[],"segments":[],"notes":"x",
        "actionItems":[{"title":"t","description":"d","segment":0}]}"#;
    assert!(matches!(
        SampleMeeting::parse(item),
        Err(SampleError::Invalid(_))
    ));
}

#[tokio::test]
async fn import_writes_a_finished_meeting_with_transcript_notes_and_items() {
    let database = test_db().await;
    let ports = ports(&database);
    let repo = SqliteMeetingRepo::new(database.clone());
    let segments = SqliteSegmentRepo::new(database.clone());
    let id = SampleImporter::new(ports).import().await.unwrap();

    let sample = SampleMeeting::bundled().unwrap();
    let state = repo.state(&id).await.unwrap().unwrap();
    assert_eq!(state.status, MeetingStatus::Ready);
    assert!(state.duration.is_some());
    assert_eq!(
        segments.count(&id).await.unwrap() as usize,
        sample.segments.len()
    );
    assert_eq!(
        segments.speakers(&id).await.unwrap().len(),
        sample.speakers.len()
    );
    let notes = SqliteEnhancedNotesRepo::new(database.clone())
        .latest(&id)
        .await
        .unwrap();
    assert!(notes.unwrap().markdown.contains("Decisions"));
    let items = count(
        &database,
        "SELECT COUNT(*) FROM action_items WHERE meeting_id = ?",
    )
    .await;
    assert_eq!(items, 3);
}

#[tokio::test]
async fn importing_twice_keeps_one_sample_and_restores_it_from_the_trash() {
    let database = test_db().await;
    let first = SampleImporter::new(ports(&database))
        .import()
        .await
        .unwrap();
    let repo = SqliteMeetingRepo::new(database.clone());
    repo.trash(&first, at("2026-10-11T10:00:00Z"))
        .await
        .unwrap();

    let second = SampleImporter::new(ports(&database))
        .import()
        .await
        .unwrap();

    assert_eq!(first, second);
    assert!(
        repo.state(&second)
            .await
            .unwrap()
            .unwrap()
            .deleted_at
            .is_none()
    );
    let meetings = count(&database, "SELECT COUNT(*) FROM meetings WHERE id = ?").await;
    let segments = count(
        &database,
        "SELECT COUNT(*) FROM transcript_segments WHERE meeting_id = ?",
    );
    let items = count(
        &database,
        "SELECT COUNT(*) FROM action_items WHERE meeting_id = ?",
    )
    .await;
    assert_eq!((meetings, items), (1, 3));
    let expected = SampleMeeting::bundled().unwrap().segments.len();
    assert_eq!(usize::try_from(segments.await).unwrap(), expected);
}

/// Notes storage that always fails, to interrupt an import half-way.
struct BrokenNotes;

#[async_trait]
impl EnhancedNotesWriter for BrokenNotes {
    async fn insert(&self, _: &NewVersion) -> Result<u32, DbError> {
        Err(sqlx::Error::PoolClosed.into())
    }

    async fn save_edit(&self, _: &VersionEdit) -> Result<bool, DbError> {
        Err(sqlx::Error::PoolClosed.into())
    }
}

#[tokio::test]
async fn a_failed_import_leaves_nothing_behind() {
    let database = test_db().await;
    let ports = SamplePorts {
        notes: Arc::new(BrokenNotes),
        ..ports(&database)
    };

    let result = SampleImporter::new(ports).import().await;

    assert!(matches!(result, Err(SampleError::Write(_))));
    let id = MeetingId::new(SAMPLE_MEETING_ID);
    let repo = SqliteMeetingRepo::new(database.clone());
    assert!(repo.state(&id).await.unwrap().is_none());
    let segments = count(
        &database,
        "SELECT COUNT(*) FROM transcript_segments WHERE meeting_id = ?",
    );
    assert_eq!(segments.await, 0);
}
