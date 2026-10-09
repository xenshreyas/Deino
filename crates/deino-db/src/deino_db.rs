//! Deino's SQLite substrate.
//!
//! - [`Database`] opens `deino.sqlite` the way every caller needs it: WAL
//!   mode, foreign keys on, one writer connection and a read-only pool.
//! - [`MIGRATOR`] embeds `migrations/`, the one schema history for the app.
//! - [`ChangeNotifier`] turns SQLite commit hooks into [`TableChanges`]
//!   signals, which the app forwards to the UI as `DbChanged` events so
//!   TanStack Query refetches exactly what changed.
//! - `test_db()` (feature `test-support`) gives repository tests an in-memory
//!   database with every migration applied.
//!
//! SQL for features lives in `deino-core/**/repo.rs`; this crate only owns
//! connections, the schema history and change signals (see README.md).

mod change_notifier;
mod database;
mod error;
mod table;

pub use change_notifier::{ChangeNotifier, TableChanges};
pub use database::{Database, DbLocation};
pub use error::DbError;
pub use table::Table;

/// Every migration in `migrations/`, embedded at compile time.
///
/// Migrations are append-only: never edit an applied file, because SQLite
/// databases in the wild record each file's checksum.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Opens an in-memory database with every migration applied, for tests.
///
/// It runs on one connection (each in-memory connection is its own empty
/// database), with foreign keys on and change hooks installed, so it behaves
/// like the real database for queries, constraints and `DbChanged` signals.
///
/// # Panics
///
/// When a migration fails to apply, which fails the calling test at once.
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test helper: a broken schema must fail the test"
)]
pub async fn test_db() -> Database {
    Database::open(&DbLocation::Memory, &MIGRATOR)
        .await
        .expect("every migration applies to an empty database")
}

#[cfg(test)]
mod database_tests;
