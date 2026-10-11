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
    /// A Rust data migration (`DataMigration`) failed.
    #[error("could not move data to its new home while updating the database")]
    DataMigration(#[source] crate::DataMigrationError),
    /// A query failed.
    #[error("a database query failed")]
    Query(#[from] sqlx::Error),
    /// There is no database file to open read-only.
    #[error("no Deino database at {}", .0.display())]
    Missing(std::path::PathBuf),
    /// The file opened read-only has no migration history.
    #[error("the file is not a Deino database")]
    NotDeino(#[source] sqlx::Error),
    /// The file's schema is not the one this binary was built for.
    #[error(transparent)]
    Schema(crate::SchemaCheck),
}

impl DbError {
    /// True when SQLite reports the file as corrupt or not a database.
    pub fn is_corruption(&self) -> bool {
        let source = match self {
            Self::Open(error)
            | Self::Query(error)
            | Self::NotDeino(error)
            | Self::Migrate(sqlx::migrate::MigrateError::Execute(error)) => error,
            Self::CreateDir(_)
            | Self::Migrate(_)
            | Self::DataMigration(_)
            | Self::Missing(_)
            | Self::Schema(_) => return false,
        };
        primary_code(source).is_some_and(|code| code == SQLITE_CORRUPT || code == SQLITE_NOTADB)
    }

    /// True when a query broke a `UNIQUE` or primary-key constraint, for
    /// example a second folder with a sibling's name. Services turn it into
    /// a `conflict` the user can fix.
    pub fn is_unique_violation(&self) -> bool {
        self.database_error()
            .is_some_and(|error| error.kind() == sqlx::error::ErrorKind::UniqueViolation)
    }

    /// True when a query referenced a row that does not exist (a foreign key
    /// to a deleted folder or tag, for example).
    pub fn is_foreign_key_violation(&self) -> bool {
        self.database_error()
            .is_some_and(|error| error.kind() == sqlx::error::ErrorKind::ForeignKeyViolation)
    }

    fn database_error(&self) -> Option<&dyn sqlx::error::DatabaseError> {
        match self {
            Self::Query(error) => error.as_database_error(),
            _ => None,
        }
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

#[cfg(test)]
mod tests {
    use crate::test_db;

    const TAG: &str = "INSERT INTO tags (id, name, color, created_at, updated_at)
        VALUES (?1, 'Sales', 1, '2026-10-09T00:00:00Z', '2026-10-09T00:00:00Z')";
    const TAG_ID: &str = "0199f000-0000-7000-8000-000000000001";

    #[tokio::test]
    async fn a_duplicate_name_is_a_unique_violation_and_nothing_else() {
        let database = test_db().await;
        sqlx::query(TAG)
            .bind(TAG_ID)
            .execute(database.writer())
            .await
            .unwrap();

        let duplicate = sqlx::query(TAG)
            .bind("0199f000-0000-7000-8000-000000000002")
            .execute(database.writer())
            .await
            .map_err(super::DbError::from)
            .unwrap_err();

        assert!(duplicate.is_unique_violation());
        assert!(!duplicate.is_foreign_key_violation());
    }

    #[tokio::test]
    async fn a_missing_parent_row_is_a_foreign_key_violation() {
        let database = test_db().await;

        let orphan = sqlx::query(
            "INSERT INTO meeting_tags (meeting_id, tag_id, applied_by, applied_at)
             VALUES ('meeting-x', ?1, 'user', '2026-10-09T00:00:00Z')",
        )
        .bind(TAG_ID)
        .execute(database.writer())
        .await
        .map_err(super::DbError::from)
        .unwrap_err();

        assert!(orphan.is_foreign_key_violation());
        assert!(!orphan.is_unique_violation());
    }
}
