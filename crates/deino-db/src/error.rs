//! What can go wrong when opening or querying the database.

use deino_domain::{ErrorClass, ErrorCode};

/// SQLite's primary result code for a corrupt database file.
const SQLITE_CORRUPT: i32 = 11;
/// SQLite's primary result code for a file that is not a database.
const SQLITE_NOTADB: i32 = 26;

/// A database failure. Messages are user-safe; the source carries the detail.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DbError {
    /// The directory that holds the database file could not be created.
    #[error("could not create the database folder")]
    CreateDir(#[source] std::io::Error),
    /// SQLite could not open or configure the database.
    #[error("could not open the database")]
    Open(#[source] sqlx::Error),
    /// A migration failed to apply.
    #[error("could not update the database schema")]
    Migrate(#[source] sqlx::migrate::MigrateError),
    /// A query failed.
    #[error("a database query failed")]
    Query(#[from] sqlx::Error),
}

impl DbError {
    /// True when SQLite reports the file as corrupt or not a database.
    pub fn is_corruption(&self) -> bool {
        let source = match self {
            Self::Open(error)
            | Self::Query(error)
            | Self::Migrate(sqlx::migrate::MigrateError::Execute(error)) => error,
            Self::CreateDir(_) | Self::Migrate(_) => return false,
        };
        primary_code(source).is_some_and(|code| code == SQLITE_CORRUPT || code == SQLITE_NOTADB)
    }
}

/// The primary SQLite result code of a database error (the low byte of an
/// extended code such as `SQLITE_CORRUPT_VTAB`).
fn primary_code(error: &sqlx::Error) -> Option<i32> {
    let code = error.as_database_error()?.code()?;
    code.parse::<i32>().ok().map(|extended| extended & 0xff)
}

impl ErrorClass for DbError {
    fn code(&self) -> ErrorCode {
        ErrorCode::Internal
    }
}
