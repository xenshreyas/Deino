//! A semantic index over a test database and the fake embedder, shared by
//! the indexer, engine and backfill tests.

use std::sync::Arc;

use deino_db::Database;
use deino_domain::{Clock, FakeClock, MeetingId};
use deino_testkit::{at, seed, test_db};

use super::indexer::IndexerParts;
use super::{SemanticIndex, SemanticIndexer, SharedVectors, SqliteEmbeddingStore};
use crate::search::FakeEmbedder;

/// Everything a semantic test needs, wired over one database.
pub(in crate::search) struct Harness {
    pub database: Database,
    pub embedder: Arc<FakeEmbedder>,
    pub store: Arc<SqliteEmbeddingStore>,
    pub vectors: Arc<SharedVectors>,
    pub indexer: Arc<SemanticIndexer>,
}

impl Harness {
    pub async fn new() -> Self {
        let database = test_db().await;
        let embedder = Arc::new(FakeEmbedder::default());
        let store = Arc::new(SqliteEmbeddingStore::new(&database));
        let vectors = Arc::new(SharedVectors::new(store.clone(), "fake-embedder"));
        let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-10T09:00:00Z")));
        let indexer = Arc::new(SemanticIndexer::new(IndexerParts {
            sources: store.clone(),
            store: store.clone(),
            embedder: embedder.clone(),
            vectors: vectors.clone(),
            clock,
        }));
        Self {
            database,
            embedder,
            store,
            vectors,
            indexer,
        }
    }

    /// The search over this harness's index.
    pub fn search(&self) -> SemanticIndex {
        SemanticIndex::new(
            self.store.clone(),
            self.embedder.clone(),
            self.vectors.clone(),
        )
    }

    /// A finished meeting whose transcript is `lines` (one segment each).
    pub async fn meeting(&self, id: &str, title: &str, lines: &[&str]) -> MeetingId {
        seed::meeting(&self.database, id, title, at("2026-10-08T09:00:00Z")).await;
        for (index, text) in lines.iter().enumerate() {
            let start = i64::try_from(index).unwrap() * 10_000;
            sqlx::query(
                "INSERT INTO transcript_segments (id, meeting_id, channel, start_ms, end_ms, text)
                 VALUES (?1, ?2, 'mixed', ?3, ?4, ?5)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(id)
            .bind(start)
            .bind(start + 5_000)
            .bind(*text)
            .execute(self.database.writer())
            .await
            .unwrap();
        }
        MeetingId::new(id)
    }

    /// Writes the meeting's notes.
    pub async fn notes(&self, meeting: &MeetingId, markdown: &str) {
        sqlx::query(
            "INSERT INTO meeting_notes (meeting_id, notes_markdown, created_at, updated_at)
             VALUES (?1, ?2, '', '')
             ON CONFLICT (meeting_id) DO UPDATE SET notes_markdown = excluded.notes_markdown",
        )
        .bind(meeting)
        .bind(markdown)
        .execute(self.database.writer())
        .await
        .unwrap();
    }
}
