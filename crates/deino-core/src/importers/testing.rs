//! An in-memory import world for tests here and in `deino-importers`: a
//! migrated database, the real runner and the reads a test checks.

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{Clock, FakeClock, MeetingId};

use super::ImportRunner;
use crate::meetings::NoMeetingWork;
use crate::search::{Hit, Retriever, SearchFilters};
use crate::tags::{RecordedTagEvents, SqliteTagRepo, TagPorts, TagService};

/// The database and a runner over it.
pub struct ImportWorld {
    /// The test database.
    pub database: Database,
    /// The runner writing into it.
    pub runner: ImportRunner,
}

impl ImportWorld {
    /// A fresh world at a fixed time.
    pub async fn new() -> Self {
        let database = test_db().await;
        let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(
            chrono::DateTime::parse_from_rfc3339("2026-10-12T09:00:00Z")
                .map(|at| at.to_utc())
                .unwrap_or_default(),
        ));
        let tag_repo = Arc::new(SqliteTagRepo::new(database.clone()));
        let tags = Arc::new(TagService::new(TagPorts {
            tags: tag_repo.clone(),
            applied: tag_repo,
            events: Arc::new(RecordedTagEvents::default()),
            clock: clock.clone(),
        }));
        let runner = ImportRunner::sqlite(&database, tags, Arc::new(NoMeetingWork), clock);
        Self { database, runner }
    }

    /// How many meetings exist.
    pub async fn meeting_count(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM meetings")
            .fetch_one(self.database.reader())
            .await
            .unwrap_or(-1)
    }

    /// Every meeting ID with its title, oldest first.
    pub async fn titles(&self) -> Vec<(MeetingId, String)> {
        sqlx::query_as("SELECT id, title FROM meetings ORDER BY created_at, id")
            .fetch_all(self.database.reader())
            .await
            .unwrap_or_default()
    }

    /// What search (and the Ask panel's retrieval) finds for `query`.
    pub async fn search(&self, query: &str) -> Vec<Hit> {
        Retriever::open(self.database.reader().clone(), None)
            .hybrid_search(query, &SearchFilters::default(), 10)
            .await
            .unwrap_or_default()
    }
}
