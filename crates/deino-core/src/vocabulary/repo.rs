//! The vocabulary's storage port and its SQLite adapter: entries in
//! `vocab_terms`, their variants in `vocab_variants`, written together.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::{VocabKind, VocabTermId};
use sqlx::SqliteConnection;

use super::VocabTerm;

/// Where vocabulary entries are stored.
#[async_trait]
pub trait VocabRepo: Send + Sync {
    /// Every entry by kind (replacements, then terms), then by spelling
    /// ignoring case; variants in the order given.
    async fn list(&self) -> Result<Vec<VocabTerm>, DbError>;

    /// Adds an entry with its variants. Fails with a unique violation when
    /// an entry of the same kind has the same spelling, ignoring case.
    async fn create(&self, term: &VocabTerm) -> Result<(), DbError>;

    /// Replaces an entry's kind, spelling, variants and `updated_at`; false
    /// when it does not exist.
    async fn update(&self, term: &VocabTerm) -> Result<bool, DbError>;

    /// Deletes an entry and its variants; false when it does not exist.
    async fn delete(&self, id: &VocabTermId) -> Result<bool, DbError>;
}

/// [`VocabRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteVocabRepo {
    database: Database,
}

impl SqliteVocabRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// One `vocab_terms` row.
struct TermRow {
    id: VocabTermId,
    kind: VocabKind,
    text: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// One `vocab_variants` row.
struct VariantRow {
    term_id: VocabTermId,
    variant: String,
}

#[async_trait]
impl VocabRepo for SqliteVocabRepo {
    async fn list(&self) -> Result<Vec<VocabTerm>, DbError> {
        let terms = sqlx::query_as!(
            TermRow,
            r#"SELECT id AS "id: VocabTermId", kind AS "kind: VocabKind", text,
                      created_at AS "created_at: DateTime<Utc>",
                      updated_at AS "updated_at: DateTime<Utc>"
               FROM vocab_terms
               ORDER BY kind, text COLLATE NOCASE"#
        )
        .fetch_all(self.database.reader())
        .await?;
        let variants = sqlx::query_as!(
            VariantRow,
            r#"SELECT term_id AS "term_id: VocabTermId", variant
               FROM vocab_variants
               ORDER BY term_id, position"#
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(assemble(terms, variants))
    }

    async fn create(&self, term: &VocabTerm) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        sqlx::query!(
            "INSERT INTO vocab_terms (id, kind, text, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            term.id,
            term.kind,
            term.text,
            term.created_at,
            term.updated_at,
        )
        .execute(&mut *transaction)
        .await?;
        insert_variants(&mut transaction, term).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn update(&self, term: &VocabTerm) -> Result<bool, DbError> {
        let mut transaction = self.database.writer().begin().await?;
        let updated = sqlx::query!(
            "UPDATE vocab_terms SET kind = ?1, text = ?2, updated_at = ?3 WHERE id = ?4",
            term.kind,
            term.text,
            term.updated_at,
            term.id,
        )
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            > 0;
        if updated {
            sqlx::query!("DELETE FROM vocab_variants WHERE term_id = ?1", term.id)
                .execute(&mut *transaction)
                .await?;
            insert_variants(&mut transaction, term).await?;
        }
        transaction.commit().await?;
        Ok(updated)
    }

    async fn delete(&self, id: &VocabTermId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM vocab_terms WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// Inserts `term`'s variants, numbered in order, inside the caller's transaction.
async fn insert_variants(
    connection: &mut SqliteConnection,
    term: &VocabTerm,
) -> Result<(), DbError> {
    for (position, variant) in (0_i64..).zip(&term.sounds_like) {
        sqlx::query!(
            "INSERT INTO vocab_variants (term_id, position, variant) VALUES (?1, ?2, ?3)",
            term.id,
            position,
            variant,
        )
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

/// Each entry with its variants, in the entries' order.
fn assemble(terms: Vec<TermRow>, variants: Vec<VariantRow>) -> Vec<VocabTerm> {
    let mut by_term: HashMap<VocabTermId, Vec<String>> = HashMap::new();
    for row in variants {
        by_term.entry(row.term_id).or_default().push(row.variant);
    }
    terms
        .into_iter()
        .map(|row| VocabTerm {
            sounds_like: by_term.remove(&row.id).unwrap_or_default(),
            id: row.id,
            kind: row.kind,
            text: row.text,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect()
}

#[cfg(test)]
#[path = "repo_tests.rs"]
mod tests;
