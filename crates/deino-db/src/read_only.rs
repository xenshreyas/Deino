//! Opening the app's database from another process, to read it only
//! (DEV-02/04, ADR 0033): the `deino` CLI and MCP server read the database
//! directly, whether or not the app is running.
//!
//! [`Database::open_read_only`] never creates, migrates or writes the file:
//! every connection is opened `SQLITE_OPEN_READONLY` (`mode=ro`) and
//! `query_only`, in WAL mode beside the app's own writer, with the same busy
//! timeout. Before it hands the database out it compares the migrations the
//! file has applied with the ones this binary embeds ([`SchemaCheck`]), so
//! a binary never reads a schema it was not compiled against.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use crate::database::{BUSY_TIMEOUT, READ_POOL_SIZE};
use crate::{ChangeNotifier, Database, DbError};

/// The table sqlx records applied migrations in.
const APPLIED_VERSIONS: &str = "SELECT version FROM _sqlx_migrations WHERE success = 1";

/// How a database's schema compares with the one a binary was built for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SchemaCheck {
    /// The database has migrations this binary does not know: the app was
    /// updated after the binary was built.
    #[error(
        "the Deino database is newer than this binary understands \
         ({unknown} unknown migration(s)); reinstall the CLI from the updated app \
         (Settings → Developer → Install CLI)"
    )]
    Newer {
        /// Applied migrations the binary does not embed.
        unknown: usize,
    },
    /// The database lacks migrations this binary expects: the app has not
    /// opened it since it was updated.
    #[error(
        "the Deino database is older than this binary expects \
         ({missing} migration(s) not applied yet); open the Deino app once to upgrade it"
    )]
    Older {
        /// Embedded migrations the database has not applied.
        missing: usize,
    },
}

impl SchemaCheck {
    /// Compares the versions a database `applied` with the ones a binary
    /// `knows`. A database with both unknown and missing migrations is
    /// reported as newer: only a newer binary can read it.
    pub fn compare(applied: &BTreeSet<i64>, knows: &BTreeSet<i64>) -> Result<(), Self> {
        let unknown = applied.difference(knows).count();
        if unknown > 0 {
            return Err(Self::Newer { unknown });
        }
        let missing = knows.difference(applied).count();
        if missing > 0 {
            return Err(Self::Older { missing });
        }
        Ok(())
    }
}

impl Database {
    /// Opens the existing database at `path` to read it only, after checking
    /// that its applied migrations are exactly `migrator`'s.
    ///
    /// Fails with [`DbError::Missing`] when there is no file (nothing is
    /// created), and with [`DbError::Schema`] when the schema differs.
    /// [`Database::writer`] and [`Database::reader`] are the same read-only
    /// pool, so a write through either fails with `SQLITE_READONLY`.
    pub async fn open_read_only(path: &Path, migrator: &Migrator) -> Result<Self, DbError> {
        if !path.is_file() {
            return Err(DbError::Missing(path.to_path_buf()));
        }
        let pool = read_only_pool(path.to_path_buf()).await?;
        let applied = applied_versions(&pool).await?;
        SchemaCheck::compare(&applied, &known_versions(migrator)).map_err(DbError::Schema)?;
        Ok(Self::from_read_only(pool, ChangeNotifier::new()))
    }
}

/// A pool of `mode=ro`, `query_only` connections in WAL mode.
async fn read_only_pool(path: PathBuf) -> Result<SqlitePool, DbError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .create_if_missing(false)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(BUSY_TIMEOUT)
        .pragma("query_only", "ON");
    SqlitePoolOptions::new()
        .max_connections(READ_POOL_SIZE)
        .connect_with(options)
        .await
        .map_err(DbError::Open)
}

/// The versions the database has applied; a file without sqlx's table is
/// not a Deino database.
async fn applied_versions(pool: &SqlitePool) -> Result<BTreeSet<i64>, DbError> {
    let versions: Vec<i64> = sqlx::query_scalar(APPLIED_VERSIONS)
        .fetch_all(pool)
        .await
        .map_err(DbError::NotDeino)?;
    Ok(versions.into_iter().collect())
}

/// The versions of `migrator`'s up migrations.
fn known_versions(migrator: &Migrator) -> BTreeSet<i64> {
    migrator
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| migration.version)
        .collect()
}

#[cfg(test)]
#[path = "read_only_tests.rs"]
mod tests;
