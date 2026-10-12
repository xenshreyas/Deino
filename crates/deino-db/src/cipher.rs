//! SQLCipher plumbing (PRV-02, ADR 0038): telling an encrypted file from a
//! plaintext one, keying connections, and naming key failures.
//!
//! The app links SQLCipher in place of SQLite (on Unix). Without a key it
//! behaves exactly like SQLite, so one binary opens both kinds of file. A
//! keyed connection runs `PRAGMA key` first (sqlx orders it before every
//! other pragma) and has statement logging turned off, so the key never
//! reaches a log line.

use std::io::Read;
use std::path::Path;

use sqlx::ConnectOptions;
use sqlx::Connection;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};

use crate::{DbError, DbKey};

/// The first 16 bytes of every plaintext SQLite database. An SQLCipher file
/// starts with its random salt instead.
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

/// SQLite's primary result code for a file that is not a database, which is
/// what SQLCipher reports for a wrong key.
const SQLITE_NOTADB: i32 = 26;

/// What is on disk at a database path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileFormat {
    /// No file yet.
    Missing,
    /// A plain SQLite file (or an empty one, which SQLite treats as new).
    Plaintext,
    /// Anything else: an SQLCipher file, readable only with its key.
    Encrypted,
}

impl FileFormat {
    /// Reads the first bytes of the file at `path`.
    pub fn detect(path: &Path) -> Result<Self, DbError> {
        let mut file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::Missing);
            }
            Err(error) => return Err(DbError::Inspect(error)),
        };
        let mut header = Vec::with_capacity(SQLITE_HEADER.len());
        file.by_ref()
            .take(SQLITE_HEADER.len() as u64)
            .read_to_end(&mut header)
            .map_err(DbError::Inspect)?;
        let plaintext = header.is_empty() || header.as_slice() == SQLITE_HEADER.as_slice();
        Ok(if plaintext {
            Self::Plaintext
        } else {
            Self::Encrypted
        })
    }

    /// Fails when opening a file of this format with (or without) `key`
    /// cannot work: an encrypted file needs a key, a plaintext one must not
    /// get one (SQLCipher would report it as "not a database").
    pub(crate) fn check_key(self, key: Option<&DbKey>) -> Result<(), DbError> {
        match (self, key) {
            (Self::Encrypted, None) => Err(DbError::KeyRequired),
            (Self::Plaintext, Some(_)) => Err(DbError::NotEncrypted),
            _ => Ok(()),
        }
    }
}

/// True when the linked SQLite is SQLCipher, so keys take effect. On plain
/// SQLite `PRAGMA key` is silently ignored, which would write plaintext.
pub async fn cipher_available() -> Result<bool, DbError> {
    let mut connection = SqliteConnection::connect_with(&SqliteConnectOptions::new())
        .await
        .map_err(DbError::Open)?;
    let version: Option<String> = sqlx::query_scalar("PRAGMA cipher_version")
        .fetch_optional(&mut connection)
        .await?;
    connection.close().await?;
    Ok(version.is_some_and(|version| !version.is_empty()))
}

/// Fails with [`DbError::CipherUnavailable`] unless SQLCipher is linked.
pub(crate) async fn require_cipher() -> Result<(), DbError> {
    if cipher_available().await? {
        Ok(())
    } else {
        Err(DbError::CipherUnavailable)
    }
}

/// `options` keyed with `key`, with statement logging off (the pragma text
/// carries the key). Without a key, `options` unchanged.
pub(crate) fn keyed(options: SqliteConnectOptions, key: Option<&DbKey>) -> SqliteConnectOptions {
    match key {
        None => options,
        Some(key) => options
            .pragma("key", key.sql_literal().to_string())
            .disable_statement_logging(),
    }
}

/// Maps a failure to open a database: with a key, "not a database" means
/// the key is wrong, never that the file is corrupt.
pub(crate) fn open_error(error: sqlx::Error, key: Option<&DbKey>) -> DbError {
    let not_a_database = error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .and_then(|code| code.parse::<i32>().ok())
        .is_some_and(|code| code & 0xff == SQLITE_NOTADB);
    if key.is_some() && not_a_database {
        DbError::WrongKey
    } else {
        DbError::Open(error)
    }
}
