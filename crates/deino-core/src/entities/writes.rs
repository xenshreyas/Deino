//! The entity write port: single-field updates that never overwrite what
//! another writer changed meanwhile (a summary refresh while the user edits
//! their notes, a logo arriving while they rename the company).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{CompanyId, PersonId};

use super::SqliteEntityRepo;

/// Saves one field of a person or company. Each returns `false`, changing
/// nothing, when no row has the ID.
#[async_trait]
pub trait EntityWrites: Send + Sync {
    /// Stores the person's AI summary, refreshed at `at`.
    async fn set_person_summary(
        &self,
        id: &PersonId,
        summary: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Stores the company's AI summary, refreshed at `at`.
    async fn set_company_summary(
        &self,
        id: &CompanyId,
        summary: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Stores the user's notes about the person.
    async fn set_person_notes(
        &self,
        id: &PersonId,
        notes: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Stores the user's notes about the company.
    async fn set_company_notes(
        &self,
        id: &CompanyId,
        notes: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;

    /// Records a favicon fetch at `at`: the cached file, or `None` when the
    /// site had none (the previous logo, if any, is kept).
    async fn set_company_logo(
        &self,
        id: &CompanyId,
        logo_path: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError>;
}

#[async_trait]
impl EntityWrites for SqliteEntityRepo {
    async fn set_person_summary(
        &self,
        id: &PersonId,
        summary: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE people SET summary_md = ?1, summary_updated_at = ?2 WHERE id = ?3",
            summary,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_company_summary(
        &self,
        id: &CompanyId,
        summary: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE companies SET summary_md = ?1, summary_updated_at = ?2 WHERE id = ?3",
            summary,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_person_notes(
        &self,
        id: &PersonId,
        notes: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE people SET notes_md = ?1, updated_at = ?2 WHERE id = ?3",
            notes,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_company_notes(
        &self,
        id: &CompanyId,
        notes: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE companies SET notes_md = ?1, updated_at = ?2 WHERE id = ?3",
            notes,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_company_logo(
        &self,
        id: &CompanyId,
        logo_path: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE companies SET logo_path = coalesce(?1, logo_path), logo_checked_at = ?2
             WHERE id = ?3",
            logo_path,
            at,
            id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
