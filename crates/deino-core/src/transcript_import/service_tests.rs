//! [`TranscriptImporter`] over the real SQLite repositories.

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{AppError, Clock, ErrorCode, FakeClock, MeetingStatus};
use deino_testkit::at;
use serde_json::json;

use super::*;
use crate::enhance::{EnhancedNotesReader, SqliteEnhancedNotesRepo};
use crate::meetings::{MeetingLifecycleRepo, NoMeetingWork, SqliteMeetingRepo};
use crate::segments::{SegmentReader, SqliteSegmentRepo};
use crate::tags::{RecordedTagEvents, SqliteTagRepo, TagPorts};

struct World {
    importer: TranscriptImporter,
    tags: Arc<TagService>,
    database: Database,
}

async fn world() -> World {
    let database = test_db().await;
    let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z")));
    let tag_repo = Arc::new(SqliteTagRepo::new(database.clone()));
    let tags = Arc::new(TagService::new(TagPorts {
        tags: tag_repo.clone(),
        applied: tag_repo,
        events: Arc::new(RecordedTagEvents::default()),
        clock: clock.clone(),
    }));
    let importer = TranscriptImporter::new(TranscriptImportPorts {
        meetings: meetings.clone(),
        trash: meetings,
        speakers: segments.clone(),
        segments,
        notes: Arc::new(SqliteEnhancedNotesRepo::new(database.clone())),
        tags: tags.clone(),
        work: Arc::new(NoMeetingWork),
        clock,
    });
    World {
        importer,
        tags,
        database,
    }
}

fn request(value: serde_json::Value) -> TranscriptImport {
    serde_json::from_value(value).unwrap()
}

#[tokio::test]
async fn an_import_becomes_a_ready_meeting_with_speakers_notes_and_tags() {
    let World {
        importer,
        tags,
        database,
    } = world().await;
    let import = request(json!({
        "name": "Vendor call",
        "createdAt": "2026-10-01T15:00:00Z",
        "tags": [{ "name": "Customer", "description": "Calls with customers" }],
        "notes": "## Decisions\n\n- Ship Friday",
        "transcript": [
            { "speaker": "Ana", "text": "Shall we ship Friday?", "timestamp": 0.0 },
            { "speaker": "Me", "text": "Yes, Friday works.", "timestamp": 4.2 }
        ]
    }));

    let id = importer.import(&import).await.unwrap();

    let state = SqliteMeetingRepo::new(database.clone())
        .state(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.status, MeetingStatus::Ready);
    assert_eq!(state.started_at, Some(at("2026-10-01T15:00:00Z")));
    let segments = SqliteSegmentRepo::new(database.clone());
    let lines = segments.list(&id, crate::segments::Page::ALL).await.unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!((lines[0].start.get(), lines[0].end.get()), (0, 4_200));
    let notes = SqliteEnhancedNotesRepo::new(database).latest(&id).await.unwrap();
    assert!(notes.unwrap().markdown.contains("Ship Friday"));
    let applied = tags.tags_of(&id).await.unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].tag.name, "Customer");
}

#[tokio::test]
async fn an_existing_tag_is_reused_ignoring_case() {
    let World { importer, tags, .. } = world().await;
    let body = json!({
        "name": "One",
        "tags": [{ "name": "hiring" }],
        "transcript": [{ "speaker": "A", "text": "Hi", "timestamp": 1 }]
    });
    importer.import(&request(body.clone())).await.unwrap();
    let mut second = body;
    second["tags"] = json!([{ "name": "HIRING" }]);

    importer.import(&request(second)).await.unwrap();

    assert_eq!(tags.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_bad_transcript_is_refused_and_leaves_nothing_behind() {
    let World {
        importer, database, ..
    } = world().await;
    let cases = [
        json!({ "name": " ", "transcript": [] }),
        json!({ "name": "x", "transcript": [{ "speaker": "", "text": "a", "timestamp": 0 }] }),
        json!({ "name": "x", "transcript": [{ "speaker": "A", "text": "a", "timestamp": -1 }] }),
        json!({ "name": "x", "transcript": [
            { "speaker": "A", "text": "b", "timestamp": 5 },
            { "speaker": "A", "text": "a", "timestamp": 1 }
        ] }),
    ];

    for body in cases {
        let error = importer.import(&request(body)).await.unwrap_err();
        assert_eq!(AppError::from(error).code, ErrorCode::Invalid);
    }
    let meetings: i64 = sqlx::query_scalar("SELECT count(*) FROM meetings")
        .fetch_one(database.reader())
        .await
        .unwrap();
    assert_eq!(meetings, 0);
}
