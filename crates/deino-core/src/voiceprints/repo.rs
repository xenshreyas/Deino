//! The voiceprint ports and their SQLite implementation.
//!
//! [`ExemplarReader`] is all that matching needs (the `diarize` and
//! `identify_speakers` stages); [`VoiceprintRepo`] enrols, lists and
//! forgets. [`SqliteVoiceprintRepo`] implements both.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{MeetingId, PersonId, VoiceprintId};

use super::{Exemplar, ProfileRow, Voiceprint, VoiceprintSummary};
use crate::f32_blob::{decode, encode};

/// Reads the exemplars voice matching compares against.
#[async_trait]
pub trait ExemplarReader: Send + Sync {
    /// Every exemplar made by `model` of a person not merged into another,
    /// grouped by person, best first.
    async fn exemplars(&self, model: &str) -> Result<Vec<Exemplar>, DbError>;
}

/// Stores, lists and deletes voiceprints.
#[async_trait]
pub trait VoiceprintRepo: Send + Sync {
    /// Adds `print`, then keeps only its person's best `keep` exemplars, in
    /// one transaction. Returns the evicted IDs (possibly `print`'s own,
    /// when it is the weakest).
    async fn enrol(&self, print: &Voiceprint, keep: usize) -> Result<Vec<VoiceprintId>, DbError>;

    /// Every unmerged person's exemplars, by name, best first.
    async fn profile_rows(&self) -> Result<Vec<ProfileRow>, DbError>;

    /// Deletes one exemplar.
    async fn delete(&self, id: &VoiceprintId) -> Result<bool, DbError>;

    /// Deletes every exemplar of `person`; how many there were.
    async fn forget(&self, person: &PersonId) -> Result<u64, DbError>;
}

/// [`ExemplarReader`] and [`VoiceprintRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteVoiceprintRepo {
    database: Database,
}

impl SqliteVoiceprintRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl ExemplarReader for SqliteVoiceprintRepo {
    async fn exemplars(&self, model: &str) -> Result<Vec<Exemplar>, DbError> {
        let rows = sqlx::query_file!("src/voiceprints/queries/exemplars.sql", model)
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| Exemplar {
                person_id: row.person_id,
                is_me: row.is_me,
                embedding: decode(&row.embedding),
            })
            .collect())
    }
}

#[async_trait]
impl VoiceprintRepo for SqliteVoiceprintRepo {
    async fn enrol(&self, print: &Voiceprint, keep: usize) -> Result<Vec<VoiceprintId>, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let embedding = encode(print.embedding.values());
        let model = print.embedding.model();
        let quality = print.quality.get();
        sqlx::query!(
            "INSERT INTO voiceprints (id, person_id, embedding, embedding_model, source_meeting_id, quality, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            print.id,
            print.person_id,
            embedding,
            model,
            print.source_meeting_id,
            quality,
            print.created_at,
        )
        .execute(&mut *transaction)
        .await?;
        let keep = i64::try_from(keep).unwrap_or(i64::MAX);
        let surplus =
            sqlx::query_file_scalar!("src/voiceprints/queries/surplus.sql", print.person_id, keep)
                .fetch_all(&mut *transaction)
                .await?;
        for id in &surplus {
            sqlx::query!("DELETE FROM voiceprints WHERE id = ?1", id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(surplus)
    }

    async fn profile_rows(&self) -> Result<Vec<ProfileRow>, DbError> {
        let rows = sqlx::query_file!("src/voiceprints/queries/profiles.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| ProfileRow {
                person_id: row.person_id,
                name: row.name,
                is_me: row.is_me,
                voiceprint: VoiceprintSummary {
                    id: row.id,
                    source_meeting_id: row.source_meeting_id,
                    source_meeting_title: row.source_meeting_title,
                    quality: row.quality,
                    created_at: row.created_at,
                },
            })
            .collect())
    }

    async fn delete(&self, id: &VoiceprintId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM voiceprints WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn forget(&self, person: &PersonId) -> Result<u64, DbError> {
        let result = sqlx::query!("DELETE FROM voiceprints WHERE person_id = ?1", person)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected())
    }
}
