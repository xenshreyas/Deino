//! Rust steps that run between two schema migrations.
//!
//! Most migrations are SQL. Some data can only be moved by Rust, because it
//! leaves the database: FND-06 moves API keys from the legacy `settings`
//! table into the keychain before a migration drops that table. Such a step
//! is a [`DataMigration`]: [`Database::open_with`] applies every schema
//! migration older than the step's [`DataMigration::runs_before`] version,
//! runs the step against that schema, then applies the rest.

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use sqlx::error::BoxDynError;
use sqlx::migrate::{Migration, MigrationSource, Migrator};

use crate::{Database, DbError};

/// What a failed step reports; it becomes the source of [`DbError::DataMigration`].
pub type DataMigrationError = Box<dyn std::error::Error + Send + Sync>;

/// A Rust step between two schema migrations.
#[async_trait]
pub trait DataMigration: Send + Sync {
    /// The version of the schema migration this step must precede, typically
    /// the one that drops the tables it reads.
    fn runs_before(&self) -> i64;

    /// Moves the data. It runs only while migration [`Self::runs_before`] is
    /// unapplied, so once per database, but a crash between the step and that
    /// migration runs it again: it must be idempotent.
    async fn run(&self, database: &Database) -> Result<(), DataMigrationError>;
}

impl Database {
    /// Applies every migration of `migrator` older than `version`, unless
    /// `version` itself is already applied. Returns whether the schema is now
    /// exactly "just before `version`", which is when a step may run.
    pub(crate) async fn migrate_before(
        &self,
        migrator: &Migrator,
        version: i64,
    ) -> Result<bool, DbError> {
        if self.is_applied(version).await? {
            return Ok(false);
        }
        let older = migrator.iter().filter(|m| m.version < version).cloned();
        let mut prefix = Migrator::new(Prefix(older.collect()))
            .await
            .map_err(DbError::Migrate)?;
        // Later migrations may exist in the file but not in this prefix.
        prefix.set_ignore_missing(true);
        prefix.run(self.writer()).await.map_err(DbError::Migrate)?;
        Ok(true)
    }

    /// True when migration `version` has been applied successfully.
    async fn is_applied(&self, version: i64) -> Result<bool, DbError> {
        let has_history: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations')",
        )
        .fetch_one(self.writer())
        .await?;
        if !has_history {
            return Ok(false);
        }
        let applied: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = ?1 AND success)",
        )
        .bind(version)
        .fetch_one(self.writer())
        .await?;
        Ok(applied)
    }
}

/// A fixed list of migrations, so a prefix of `MIGRATOR` can run on its own.
#[derive(Debug)]
struct Prefix(Vec<Migration>);

impl MigrationSource<'static> for Prefix {
    fn resolve(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Migration>, BoxDynError>> + Send + 'static>> {
        Box::pin(async move { Ok(self.0) })
    }
}
