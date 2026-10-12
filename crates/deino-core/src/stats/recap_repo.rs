//! Where generated year-in-review recaps are kept: `year_recaps`, one
//! write-once row per year.

use async_trait::async_trait;
use deino_db::{Database, DbError};

use super::YearRecap;

/// Stores recaps.
#[async_trait]
pub trait RecapRepo: Send + Sync {
    /// The recap of `year`, if one was generated.
    async fn get(&self, year: i32) -> Result<Option<YearRecap>, DbError>;

    /// Keeps `recap` unless `year` already has one, and returns the stored
    /// recap: the first one written wins, so two racing generations agree.
    async fn save(&self, recap: &YearRecap) -> Result<YearRecap, DbError>;
}

/// [`RecapRepo`] over SQLite.
#[derive(Clone, Debug)]
pub struct SqliteRecapRepo {
    database: Database,
}

impl SqliteRecapRepo {
    /// A repository over the app database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl RecapRepo for SqliteRecapRepo {
    async fn get(&self, year: i32) -> Result<Option<YearRecap>, DbError> {
        let recap = sqlx::query_as!(
            YearRecap,
            r#"SELECT year AS "year!: i32", markdown, generated_at AS "generated_at: _"
               FROM year_recaps WHERE year = ?1"#,
            year,
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(recap)
    }

    async fn save(&self, recap: &YearRecap) -> Result<YearRecap, DbError> {
        let stored = sqlx::query_as!(
            YearRecap,
            r#"INSERT INTO year_recaps (year, markdown, generated_at) VALUES (?1, ?2, ?3)
               ON CONFLICT (year) DO UPDATE SET year = year
               RETURNING year AS "year!: i32", markdown, generated_at AS "generated_at: _""#,
            recap.year,
            recap.markdown,
            recap.generated_at,
        )
        .fetch_one(self.database.writer())
        .await?;
        Ok(stored)
    }
}
