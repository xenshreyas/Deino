//! The company port and its SQLite implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::{Database, DbError};
use deino_domain::CompanyId;

use super::{Company, CompanyDomain};

/// Where companies are stored.
#[async_trait]
pub trait CompanyRepo: Send + Sync {
    /// Adds a company. Fails if another company has its domain.
    async fn create(&self, company: &Company) -> Result<(), DbError>;

    /// The company with `id`.
    async fn find(&self, id: &CompanyId) -> Result<Option<Company>, DbError>;

    /// The company with `domain`, for upserting people by email domain.
    async fn find_by_domain(&self, domain: &CompanyDomain) -> Result<Option<Company>, DbError>;

    /// Every company, by name (case-insensitive).
    async fn list(&self) -> Result<Vec<Company>, DbError>;

    /// Saves every editable field of `company` and its `updated_at`.
    async fn update(&self, company: &Company) -> Result<bool, DbError>;

    /// Deletes the company; its people stay, without a company.
    async fn delete(&self, id: &CompanyId) -> Result<bool, DbError>;
}

/// [`CompanyRepo`] over Deino's SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteCompanyRepo {
    pub(super) database: Database,
}

impl SqliteCompanyRepo {
    /// Creates a repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl CompanyRepo for SqliteCompanyRepo {
    async fn create(&self, company: &Company) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO companies (id, name, domain, logo_path, notes_md, summary_md, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            company.id,
            company.name,
            company.domain,
            company.logo_path,
            company.notes_md,
            company.summary_md,
            company.created_at,
            company.updated_at,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }

    async fn find(&self, id: &CompanyId) -> Result<Option<Company>, DbError> {
        let company = sqlx::query_as!(
            Company,
            r#"SELECT id AS "id: CompanyId", name, domain AS "domain: CompanyDomain", logo_path,
                notes_md, summary_md, created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>"
            FROM companies WHERE id = ?1"#,
            id,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(company)
    }

    async fn find_by_domain(&self, domain: &CompanyDomain) -> Result<Option<Company>, DbError> {
        let company = sqlx::query_as!(
            Company,
            r#"SELECT id AS "id: CompanyId", name, domain AS "domain: CompanyDomain", logo_path,
                notes_md, summary_md, created_at AS "created_at: DateTime<Utc>",
                updated_at AS "updated_at: DateTime<Utc>"
            FROM companies WHERE domain = ?1"#,
            domain,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(company)
    }

    async fn list(&self) -> Result<Vec<Company>, DbError> {
        let companies = sqlx::query_file_as!(Company, "src/companies/queries/list.sql")
            .fetch_all(self.database.reader())
            .await?;
        Ok(companies)
    }

    async fn update(&self, company: &Company) -> Result<bool, DbError> {
        let result = sqlx::query!(
            "UPDATE companies
             SET name = ?1, domain = ?2, logo_path = ?3, notes_md = ?4, summary_md = ?5, updated_at = ?6
             WHERE id = ?7",
            company.name,
            company.domain,
            company.logo_path,
            company.notes_md,
            company.summary_md,
            company.updated_at,
            company.id,
        )
        .execute(self.database.writer())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: &CompanyId) -> Result<bool, DbError> {
        let result = sqlx::query!("DELETE FROM companies WHERE id = ?1", id)
            .execute(self.database.writer())
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
