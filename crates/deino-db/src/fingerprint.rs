//! [`Fingerprint`]: every table's row count and content checksum, plus the
//! schema's, so two databases can be compared without trusting the copy
//! (PRV-02: an encryption migration is verified before it replaces the
//! original).
//!
//! Each row is rendered with SQLite's `quote()` (exact for every storage
//! class, blobs included), the rows are sorted, and the sorted text is
//! hashed with SHA-256. Sorting makes the checksum independent of storage
//! order, so a copy that rebuilt its pages still matches.

use std::collections::BTreeMap;

use futures_util::TryStreamExt;
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqliteConnection;

use crate::DbError;

/// Every table that stores rows. Virtual tables (FTS5) store theirs in shadow
/// tables, which are listed; `sqlite_stat*` only holds planner statistics.
const TABLES: &str = "SELECT name FROM sqlite_schema \
     WHERE type = 'table' AND rootpage > 0 AND name NOT LIKE 'sqlite_stat%' \
     ORDER BY name";

/// The schema itself, as one pseudo-table.
const SCHEMA: &str = "SELECT quote(type) || ',' || quote(name) || ',' || quote(tbl_name) \
     || ',' || quote(sql) AS line FROM sqlite_schema \
     WHERE name NOT LIKE 'sqlite_stat%' ORDER BY line";

/// A table's columns in declaration order.
const COLUMNS: &str = "SELECT name FROM pragma_table_info(?1) ORDER BY cid";

/// The key the schema's digest is filed under (no table can be named so).
pub const SCHEMA_ENTRY: &str = "<schema>";

/// One table's contents in short.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableDigest {
    /// How many rows the table holds.
    pub rows: u64,
    /// SHA-256 over the table's sorted, quoted rows.
    pub sha256: [u8; 32],
}

/// Every table's digest, by name, and the schema's under [`SCHEMA_ENTRY`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fingerprint(BTreeMap<String, TableDigest>);

impl Fingerprint {
    /// Reads every table on `connection`.
    pub async fn of(connection: &mut SqliteConnection) -> Result<Self, DbError> {
        let tables: Vec<String> = sqlx::query_scalar(TABLES)
            .fetch_all(&mut *connection)
            .await?;
        let mut digests = BTreeMap::new();
        digests.insert(SCHEMA_ENTRY.to_owned(), digest(connection, SCHEMA).await?);
        for table in tables {
            let sql = rows_sql(connection, &table).await?;
            digests.insert(table, digest(connection, &sql).await?);
        }
        Ok(Self(digests))
    }

    /// The digest of `table`, if the database has it.
    pub fn table(&self, table: &str) -> Option<&TableDigest> {
        self.0.get(table)
    }

    /// Every table name with a digest (the schema entry included).
    pub fn tables(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

/// A query returning one sorted text line per row of `table`.
async fn rows_sql(connection: &mut SqliteConnection, table: &str) -> Result<String, DbError> {
    let columns: Vec<String> = sqlx::query_scalar(COLUMNS)
        .bind(table)
        .fetch_all(&mut *connection)
        .await?;
    let line = columns
        .iter()
        .map(|column| format!("quote({})", identifier(column)))
        .collect::<Vec<_>>()
        .join(" || ',' || ");
    Ok(format!(
        "SELECT {line} AS line FROM {} ORDER BY line",
        identifier(table)
    ))
}

/// Counts and hashes the lines `sql` returns, streaming them.
async fn digest(connection: &mut SqliteConnection, sql: &str) -> Result<TableDigest, DbError> {
    let mut hasher = Sha256::new();
    let mut rows = 0_u64;
    let mut lines = sqlx::query_scalar::<_, Option<String>>(sql).fetch(connection);
    while let Some(line) = lines.try_next().await? {
        hasher.update(line.unwrap_or_default().as_bytes());
        hasher.update(b"\n");
        rows += 1;
    }
    Ok(TableDigest {
        rows,
        sha256: hasher.finalize().into(),
    })
}

/// `name` as a quoted SQL identifier.
fn identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
