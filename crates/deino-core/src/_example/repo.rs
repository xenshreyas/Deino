//! The example's storage port and its SQLite adapter.
//!
//! The port is what the service needs, phrased in domain types; the adapter
//! is SQL only. Reads use the read pool, writes the writer, and every query
//! is checked at compile time against the migrated schema.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::ExampleNoteId;

use super::ExampleNote;

/// Where notes are stored.
#[async_trait]
pub trait ExampleNoteRepo: Send + Sync {
    /// Every note, newest first.
    async fn list(&self) -> Result<Vec<ExampleNote>, DbError>;

    /// Saves a new note.
    async fn insert(&self, note: &ExampleNote) -> Result<(), DbError>;
}

/// [`ExampleNoteRepo`] over SQLite.
#[derive(Clone, Debug)]
pub struct SqliteExampleNoteRepo {
    database: Database,
}

impl SqliteExampleNoteRepo {
    /// Creates a repository over a database migrated with the example's `MIGRATOR`.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl ExampleNoteRepo for SqliteExampleNoteRepo {
    async fn list(&self) -> Result<Vec<ExampleNote>, DbError> {
        let notes = sqlx::query_file_as!(ExampleNote, "src/_example/queries/list.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(notes)
    }

    async fn insert(&self, note: &ExampleNote) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO example_notes (id, text, created_at) VALUES (?1, ?2, ?3)",
            note.id,
            note.text,
            note.created_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use deino_db::DbLocation;
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;
    use crate::_example::MIGRATOR;

    async fn repo() -> SqliteExampleNoteRepo {
        let database = Database::open(&DbLocation::Memory, &MIGRATOR);
        SqliteExampleNoteRepo::new(database.await.unwrap())
    }

    fn note(text: &str, created_at: &str) -> ExampleNote {
        let created_at = at(created_at);
        let id = ExampleNoteId::generate(&FakeClock::new(created_at));
        ExampleNote {
            id,
            text: text.into(),
            created_at,
        }
    }

    #[tokio::test]
    async fn inserted_notes_are_listed_newest_first() {
        let repo = repo().await;
        let older = note("first", "2026-10-08T09:00:00Z");
        let newer = note("second", "2026-10-08T09:05:00Z");

        repo.insert(&older).await.unwrap();
        repo.insert(&newer).await.unwrap();

        assert_eq!(repo.list().await.unwrap(), [newer, older]);
    }

    #[tokio::test]
    async fn the_schema_rejects_blank_text_even_without_the_service() {
        let repo = repo().await;

        let result = repo.insert(&note("   ", "2026-10-08T09:00:00Z")).await;

        assert!(matches!(result, Err(DbError::Query(_))));
    }

    #[tokio::test]
    async fn list_is_an_index_scan_without_a_sort() {
        let repo = repo().await;
        let explain = format!("EXPLAIN QUERY PLAN {}", include_str!("queries/list.sql"));

        // Rows are (id, parent, unused, detail); the detail names the access path.
        let steps: Vec<(i64, i64, i64, String)> = sqlx::query_as(&explain)
            .fetch_all(repo.database.reader())
            .await
            .unwrap();
        let plan: Vec<&str> = steps.iter().map(|(.., detail)| detail.as_str()).collect();

        assert_eq!(
            plan,
            ["SCAN example_notes USING INDEX idx_example_notes_created_at_id"]
        );
    }
}
