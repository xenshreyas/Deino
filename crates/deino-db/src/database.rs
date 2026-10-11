//! Opening, migrating and closing a Deino database.

use std::path::PathBuf;
use std::time::Duration;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use crate::{ChangeNotifier, DataMigration, DbError};

/// Read connections. In WAL mode SQLite serves readers alongside the writer.
pub(crate) const READ_POOL_SIZE: u32 = 4;
/// How long a statement waits for a lock before failing with `SQLITE_BUSY`.
pub(crate) const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Where a database lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DbLocation {
    /// A file, created with its parent directory if missing.
    File(PathBuf),
    /// A private in-memory database that lives as long as the [`Database`].
    Memory,
}

/// An open, migrated database: one writer connection and a read pool.
///
/// All writes go through [`Database::writer`], a single connection, so writes
/// never contend with each other and every commit passes the change hooks.
/// Reads use [`Database::reader`], whose connections are `query_only`. Clones
/// share the same pools.
#[derive(Clone, Debug)]
pub struct Database {
    writer: SqlitePool,
    reader: SqlitePool,
    changes: ChangeNotifier,
    /// False for a database opened with [`Database::open_read_only`].
    writable: bool,
}

impl Database {
    /// Opens the database at `location` and applies `migrator`'s migrations.
    ///
    /// Every commit on the writer is reported through [`Database::changes`].
    pub async fn open(location: &DbLocation, migrator: &Migrator) -> Result<Self, DbError> {
        Self::open_with(location, migrator, &[]).await
    }

    /// Opens the database like [`Database::open`], running each Rust `step`
    /// against the schema just before its [`DataMigration::runs_before`]
    /// version. Steps whose version is already applied are skipped.
    pub async fn open_with(
        location: &DbLocation,
        migrator: &Migrator,
        steps: &[&dyn DataMigration],
    ) -> Result<Self, DbError> {
        let database = Self::connect(location).await?;
        let mut ordered = steps.to_vec();
        ordered.sort_by_key(|step| step.runs_before());
        let mut moved_data = false;
        for step in ordered {
            if database
                .migrate_before(migrator, step.runs_before())
                .await?
            {
                step.run(&database).await.map_err(DbError::DataMigration)?;
                moved_data = true;
            }
        }
        migrator
            .run(&database.writer)
            .await
            .map_err(DbError::Migrate)?;
        if moved_data {
            // Data that left the database (FND-06's keys) must leave the file
            // now, not at the next clean shutdown: write the WAL back.
            database.checkpoint().await;
        }
        Ok(database)
    }

    /// Opens the writer and the read pool without touching the schema.
    pub(crate) async fn connect(location: &DbLocation) -> Result<Self, DbError> {
        if let DbLocation::File(path) = location
            && let Some(directory) = path.parent()
        {
            std::fs::create_dir_all(directory).map_err(DbError::CreateDir)?;
        }
        let changes = ChangeNotifier::new();
        let options = connect_options(location);
        let writer = changes
            .install(single_connection_pool())
            .connect_with(options.clone())
            .await
            .map_err(DbError::Open)?;
        let reader = match location {
            // Each in-memory connection is its own database, so reads share the writer.
            DbLocation::Memory => writer.clone(),
            DbLocation::File(_) => read_pool(options).await?,
        };
        Ok(Self {
            writer,
            reader,
            changes,
            writable: true,
        })
    }

    /// A database whose writer and reader are both the read-only `pool`.
    pub(crate) fn from_read_only(pool: SqlitePool, changes: ChangeNotifier) -> Self {
        Self {
            writer: pool.clone(),
            reader: pool,
            changes,
            writable: false,
        }
    }

    /// The single write connection. Use it for every `INSERT`, `UPDATE`,
    /// `DELETE` and transaction.
    pub fn writer(&self) -> &SqlitePool {
        &self.writer
    }

    /// The read-only pool. Use it for queries that do not write.
    pub fn reader(&self) -> &SqlitePool {
        &self.reader
    }

    /// The notifier that reports this database's commits.
    pub fn changes(&self) -> &ChangeNotifier {
        &self.changes
    }

    /// Checkpoints the WAL into the main file and closes every connection.
    ///
    /// Call it on shutdown so no `-wal`/`-shm` files are left behind. A
    /// read-only database only closes: the WAL belongs to the app.
    pub async fn close(&self) {
        self.reader.close().await;
        if self.writable {
            self.checkpoint().await;
        }
        self.writer.close().await;
    }

    /// Copies every WAL frame into the database file and empties the WAL
    /// (`TRUNCATE`). A failure only delays that to the next checkpoint.
    async fn checkpoint(&self) {
        let checkpoint = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.writer)
            .await;
        if let Err(error) = checkpoint {
            log::warn!(target: "db", "WAL checkpoint failed: {error}");
        }
    }
}

/// Connection settings shared by the writer and the readers.
fn connect_options(location: &DbLocation) -> SqliteConnectOptions {
    let options = match location {
        DbLocation::File(path) => SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal),
        // `SqliteConnectOptions::new()` opens a private `:memory:` database.
        DbLocation::Memory => SqliteConnectOptions::new(),
    };
    options.foreign_keys(true).busy_timeout(BUSY_TIMEOUT)
}

/// A pool that keeps exactly one connection open for its whole life, so an
/// in-memory database is never dropped and hooks are installed once.
fn single_connection_pool() -> SqlitePoolOptions {
    SqlitePoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
}

async fn read_pool(options: SqliteConnectOptions) -> Result<SqlitePool, DbError> {
    SqlitePoolOptions::new()
        .max_connections(READ_POOL_SIZE)
        .connect_with(options.pragma("query_only", "ON"))
        .await
        .map_err(DbError::Open)
}
