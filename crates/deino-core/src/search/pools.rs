//! The connections the SQLite search adapters use: reads always, writes only
//! in the app. Another process (the `deino` CLI and MCP server, DEV-02/04)
//! opens the database read-only and gets adapters without a writer.

use deino_db::Database;
use sqlx::SqlitePool;

use super::SearchError;

/// A read pool and, in the app, the writer.
#[derive(Clone, Debug)]
pub(super) struct Pools {
    reader: SqlitePool,
    writer: Option<SqlitePool>,
}

impl Pools {
    /// Both pools of the app's open database.
    pub(super) fn of(database: &Database) -> Self {
        Self {
            reader: database.reader().clone(),
            writer: Some(database.writer().clone()),
        }
    }

    /// Reads only, over a pool another process opened (read-only).
    pub(super) fn read_only(reader: SqlitePool) -> Self {
        Self {
            reader,
            writer: None,
        }
    }

    /// The pool reads use.
    pub(super) fn reader(&self) -> &SqlitePool {
        &self.reader
    }

    /// The writer, or [`SearchError::ReadOnly`] for read-only adapters.
    pub(super) fn writer(&self) -> Result<&SqlitePool, SearchError> {
        self.writer.as_ref().ok_or(SearchError::ReadOnly)
    }
}
