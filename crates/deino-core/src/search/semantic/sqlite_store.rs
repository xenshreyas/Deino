//! [`MeetingSourceReader`] and [`EmbeddingStore`] over SQLite.
//!
//! Vectors are stored as little-endian `f32` BLOBs. Notes and summaries are
//! read from the keyword index's documents, which already hold their
//! Markdown (the ORG-08 triggers keep them current).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::Database;
use deino_domain::{MeetingId, Millis};
use sqlx::SqlitePool;

use super::store::{
    ChunkRow, Coverage, EmbeddingStore, ExistingEmbedding, MeetingSourceReader, MeetingSources,
    Replacement,
};
use super::{Chunk, ChunkKind, StoredVector, TranscriptLine};
use crate::search::SearchError;
use crate::search::pools::Pools;
use crate::search::sqlite::json_array;

/// The semantic index's storage in Deino's database.
#[derive(Clone, Debug)]
pub struct SqliteEmbeddingStore {
    pools: Pools,
}

impl SqliteEmbeddingStore {
    /// A store over an open, migrated database.
    pub fn new(database: &Database) -> Self {
        Self {
            pools: Pools::of(database),
        }
    }

    /// A store that only reads, over `pool` (a read-only connection another
    /// process opened); saving vectors fails with [`SearchError::ReadOnly`].
    pub fn read_only(pool: SqlitePool) -> Self {
        Self {
            pools: Pools::read_only(pool),
        }
    }
}

#[async_trait]
impl MeetingSourceReader for SqliteEmbeddingStore {
    async fn sources(&self, meeting: &MeetingId) -> Result<MeetingSources, SearchError> {
        let transcript = sqlx::query_file_as!(
            TranscriptLine,
            "src/search/semantic/queries/transcript.sql",
            meeting
        )
        .fetch_all(self.pools.reader())
        .await?;
        let sections = sqlx::query!(
            r#"SELECT kind AS "kind!", body AS "body!" FROM search_documents
               WHERE meeting_id = ?1 AND kind IN ('note', 'summary')"#,
            meeting
        )
        .fetch_all(self.pools.reader())
        .await?;
        let section = |kind: &str| {
            sections
                .iter()
                .find(|row| row.kind == kind)
                .map(|row| row.body.clone())
        };
        Ok(MeetingSources {
            notes: section("note"),
            summary: section("summary"),
            transcript,
        })
    }

    async fn to_index(&self) -> Result<Vec<MeetingId>, SearchError> {
        let meetings = sqlx::query_file_scalar!("src/search/semantic/queries/to_index.sql")
            .fetch_all(self.pools.reader())
            .await?;
        Ok(meetings)
    }
}

#[async_trait]
impl EmbeddingStore for SqliteEmbeddingStore {
    async fn existing(&self, meeting: &MeetingId) -> Result<Vec<ExistingEmbedding>, SearchError> {
        let rows = sqlx::query!(
            r#"SELECT text AS "text!", model AS "model!", vector AS "vector!"
               FROM embeddings WHERE meeting_id = ?1"#,
            meeting
        )
        .fetch_all(self.pools.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| ExistingEmbedding {
                text: row.text,
                model: row.model,
                vector: decode(&row.vector),
            })
            .collect())
    }

    async fn replace(
        &self,
        replacement: Replacement<'_>,
    ) -> Result<Vec<StoredVector>, SearchError> {
        let mut transaction = self.pools.writer()?.begin().await?;
        sqlx::query!(
            "DELETE FROM embeddings WHERE meeting_id = ?1",
            replacement.meeting
        )
        .execute(&mut *transaction)
        .await?;
        let mut stored = Vec::with_capacity(replacement.chunks.len());
        for (chunk, vector) in replacement.chunks.iter().zip(replacement.vectors) {
            let id = insert(&mut transaction, &replacement, chunk, vector).await?;
            stored.push(StoredVector {
                id,
                meeting: replacement.meeting.clone(),
                vector: vector.clone(),
            });
        }
        transaction.commit().await?;
        Ok(stored)
    }

    async fn load(&self, model: &str) -> Result<Vec<StoredVector>, SearchError> {
        let rows = sqlx::query!(
            r#"SELECT id AS "id!", meeting_id AS "meeting_id!: MeetingId", vector AS "vector!"
               FROM embeddings WHERE model = ?1"#,
            model
        )
        .fetch_all(self.pools.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| StoredVector {
                id: row.id,
                meeting: row.meeting_id,
                vector: decode(&row.vector),
            })
            .collect())
    }

    async fn hydrate(&self, ids: &[i64]) -> Result<Vec<ChunkRow>, SearchError> {
        let ids: Vec<String> = ids.iter().map(ToString::to_string).collect();
        let ids = json_array(ids.iter().map(String::as_str));
        let rows = sqlx::query_file_as!(ChunkRow, "src/search/semantic/queries/hydrate.sql", ids)
            .fetch_all(self.pools.reader())
            .await?;
        Ok(rows)
    }

    async fn coverage(&self, model: &str) -> Result<Coverage, SearchError> {
        let row = sqlx::query_file!("src/search/semantic/queries/coverage.sql", model)
            .fetch_one(self.pools.reader())
            .await?;
        Ok(Coverage {
            indexed: row.indexed,
            total: row.total,
        })
    }
}

/// Inserts one chunk of `replacement` and returns its row ID.
async fn insert(
    connection: &mut sqlx::SqliteConnection,
    replacement: &Replacement<'_>,
    chunk: &Chunk,
    vector: &[f32],
) -> Result<i64, SearchError> {
    let (start, end): (Option<Millis>, Option<Millis>) = chunk
        .span
        .map_or((None, None), |(start, end)| (Some(start), Some(end)));
    let bytes = encode(vector);
    let id = sqlx::query_scalar!(
        r#"INSERT INTO embeddings
               (meeting_id, ref_kind, ref_id, chunk_ix, ts_ms, end_ms, text, model, vector, created_at)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
           RETURNING id AS "id!""#,
        replacement.meeting,
        chunk.kind,
        chunk.ref_id,
        chunk.index,
        start,
        end,
        chunk.text,
        replacement.model,
        bytes,
        replacement.at,
    )
    .fetch_one(&mut *connection)
    .await?;
    Ok(id)
}

/// `vector` as little-endian `f32` bytes.
fn encode(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// Little-endian `f32` bytes as a vector (a ragged tail is ignored; the
/// table's CHECK keeps the length a multiple of four).
fn decode(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|word| word.try_into().map_or(0.0, f32::from_le_bytes))
        .collect()
}

#[cfg(test)]
#[path = "sqlite_store_tests.rs"]
mod tests;
