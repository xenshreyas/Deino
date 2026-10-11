use deino_db::Database;
use deino_domain::{MeetingId, Millis};
use deino_testkit::{at, seed, test_db};

use super::*;
use crate::search::semantic::chunk::Chunk;
use crate::search::semantic::store::Replacement;
use crate::testing::query_plan;

async fn execute(database: &Database, sql: &str) {
    sqlx::query(sql).execute(database.writer()).await.unwrap();
}

async fn seeded() -> Database {
    let database = test_db().await;
    seed::meeting(
        &database,
        "meeting-a",
        "Roadmap",
        at("2026-10-08T09:00:00Z"),
    )
    .await;
    execute(
        &database,
        "INSERT INTO speakers (id, meeting_id, label, color) VALUES
        ('0192f3a0-0000-7000-8000-000000000051', 'meeting-a', 'Speaker 1', 1)",
    )
    .await;
    execute(&database, "INSERT INTO transcript_segments (id, meeting_id, channel, speaker_id, start_ms, end_ms, text, is_deleted) VALUES
        ('0192f3a0-0000-7000-8000-0000000000a2', 'meeting-a', 'mixed', NULL, 5000, 6000, 'second', 0),
        ('0192f3a0-0000-7000-8000-0000000000a1', 'meeting-a', 'mixed', '0192f3a0-0000-7000-8000-000000000051', 1000, 2000, 'first', 0),
        ('0192f3a0-0000-7000-8000-0000000000a3', 'meeting-a', 'mixed', NULL, 7000, 8000, 'gone', 1)").await;
    execute(
        &database,
        "INSERT INTO meeting_notes (meeting_id, notes_markdown, created_at, updated_at)
        VALUES ('meeting-a', '# Notes', '', '')",
    )
    .await;
    database
}

fn chunk(index: u32, text: &str) -> Chunk {
    Chunk {
        kind: ChunkKind::Transcript,
        ref_id: "0192f3a0-0000-7000-8000-0000000000a1".into(),
        index,
        span: Some((Millis(1_000), Millis(6_000))),
        text: text.into(),
    }
}

#[tokio::test]
async fn sources_are_the_live_transcript_in_order_with_speakers_and_the_notes() {
    let database = seeded().await;
    let store = SqliteEmbeddingStore::new(database);

    let sources = store.sources(&MeetingId::new("meeting-a")).await.unwrap();

    let lines: Vec<(&str, Option<&str>)> = sources
        .transcript
        .iter()
        .map(|line| (line.text.as_str(), line.speaker.as_deref()))
        .collect();
    assert_eq!(lines, [("first", Some("Speaker 1")), ("second", None)]);
    assert_eq!(sources.notes.as_deref(), Some("# Notes"));
    assert_eq!(sources.summary, None);
}

/// Replaces meeting-a's chunks with `chunks` and one-dimension-per-chunk vectors.
async fn replace(
    store: &SqliteEmbeddingStore,
    chunks: &[Chunk],
    vectors: &[Vec<f32>],
) -> Vec<StoredVector> {
    let meeting = MeetingId::new("meeting-a");
    let replacement = Replacement {
        meeting: &meeting,
        model: "model-x",
        chunks,
        vectors,
        at: at("2026-10-08T10:00:00Z"),
    };
    store.replace(replacement).await.unwrap()
}

#[tokio::test]
async fn a_replace_drops_the_old_rows_and_vectors_round_trip() {
    let store = SqliteEmbeddingStore::new(seeded().await);
    let chunks = [chunk(0, "first second"), chunk(1, "more")];
    let vectors = [vec![0.25, -1.5], vec![1.0, 0.0]];

    let stored = replace(&store, &chunks, &vectors).await;
    let again = replace(&store, &chunks[..1], &vectors[..1]).await;

    assert_eq!(stored.len(), 2);
    let loaded = store.load("model-x").await.unwrap();
    assert_eq!(loaded, again);
    assert_eq!(loaded[0].vector, [0.25, -1.5]);
    assert!(store.load("model-y").await.unwrap().is_empty());
}

#[tokio::test]
async fn hydrate_reads_chunks_with_their_meeting_and_skips_the_trash() {
    let database = seeded().await;
    let store = SqliteEmbeddingStore::new(database.clone());
    let stored = replace(&store, &[chunk(0, "first second")], &[vec![1.0]]).await;

    let rows = store.hydrate(&[stored[0].id]).await.unwrap();

    assert_eq!(rows[0].text, "first second");
    assert_eq!(rows[0].title, "Roadmap");
    assert_eq!(
        (rows[0].ts_ms, rows[0].end_ms),
        (Some(Millis(1_000)), Some(Millis(6_000)))
    );
    execute(
        &database,
        "UPDATE meetings SET deleted_at = '2026-10-09' WHERE id = 'meeting-a'",
    )
    .await;
    assert!(store.hydrate(&[stored[0].id]).await.unwrap().is_empty());
}

#[tokio::test]
async fn coverage_and_the_work_list_count_meetings_with_something_to_embed() {
    let database = seeded().await;
    seed::meeting(
        &database,
        "meeting-empty",
        "Nothing said",
        at("2026-10-09T09:00:00Z"),
    )
    .await;
    let store = SqliteEmbeddingStore::new(database);
    let meeting = MeetingId::new("meeting-a");

    assert_eq!(
        store.to_index().await.unwrap(),
        std::slice::from_ref(&meeting)
    );
    let none = Coverage {
        indexed: 0,
        total: 1,
    };
    assert_eq!(store.coverage("model-x").await.unwrap(), none);
    replace(&store, &[chunk(0, "x")], &[vec![1.0]]).await;
    let one = Coverage {
        indexed: 1,
        total: 1,
    };
    assert_eq!(store.coverage("model-x").await.unwrap(), one);
    assert_eq!(store.coverage("model-y").await.unwrap().indexed, 0);
}

#[tokio::test]
async fn coverage_reads_the_model_index_not_the_vectors() {
    let database = seeded().await;

    let plan = query_plan(&database, include_str!("queries/coverage.sql")).await;

    assert!(
        plan.iter()
            .any(|step| step.contains("COVERING INDEX idx_embeddings_model_meeting_id")),
        "{plan:?}"
    );
}
