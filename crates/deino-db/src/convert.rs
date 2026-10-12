//! Encrypting a plaintext database file, or decrypting an encrypted one, in
//! place and crash-safe (PRV-02, ADR 0038).
//!
//! [`convert`] never edits the original. It exports a copy with
//! `sqlcipher_export` into `<db>.converting`, fingerprints both
//! ([`Fingerprint`]) and only when every table matches swaps the copy in:
//! the original moves to `<db>.backup` and the copy to `<db>`, each an atomic
//! rename. The backup is deleted once the new file opens with the expected
//! key and matches again; if it does not, the backup is moved back.
//!
//! [`recover`] finishes or undoes a conversion a crash interrupted. Run it
//! before opening the database. Only the app may run either function, and
//! only while it has the database closed.

use std::path::{Path, PathBuf};

use sqlx::ConnectOptions;
use sqlx::Connection;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};

use crate::cipher::{FileFormat, keyed, open_error, require_cipher};
use crate::{DbError, DbKey, Fingerprint};

/// The schema name the copy is attached under while it is written.
const TARGET_SCHEMA: &str = "converted";

/// Which way a conversion goes, and the key it uses.
#[derive(Clone, Copy)]
pub enum Conversion<'k> {
    /// Plaintext → encrypted with the key.
    Encrypt(&'k DbKey),
    /// Encrypted with the key → plaintext.
    Decrypt(&'k DbKey),
}

impl Conversion<'_> {
    /// The key the original opens with.
    fn source_key(&self) -> Option<&DbKey> {
        match self {
            Self::Encrypt(_) => None,
            Self::Decrypt(key) => Some(key),
        }
    }

    /// The key the copy opens with.
    fn target_key(&self) -> Option<&DbKey> {
        match self {
            Self::Encrypt(key) => Some(key),
            Self::Decrypt(_) => None,
        }
    }

    /// Fails unless the original is in the format this conversion reads.
    fn check_source(&self, path: &Path, format: FileFormat) -> Result<(), DbError> {
        match (self, format) {
            (_, FileFormat::Missing) => Err(DbError::Missing(path.to_path_buf())),
            (Self::Encrypt(_), FileFormat::Encrypted) => Err(DbError::AlreadyEncrypted),
            (Self::Decrypt(_), FileFormat::Plaintext) => Err(DbError::NotEncrypted),
            _ => Ok(()),
        }
    }
}

/// What [`recover`] found and did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// No conversion was in flight.
    Clean,
    /// A copy that was never swapped in was deleted.
    DiscardedCopy,
    /// The swap had finished: the leftover backup was deleted.
    DiscardedBackup,
    /// The original had been moved aside but the copy not yet moved in:
    /// the original was put back.
    RestoredOriginal,
}

/// Converts the closed database at `path` as `conversion` says, keeping the
/// original until the converted file is verified.
pub async fn convert(path: &Path, conversion: Conversion<'_>) -> Result<(), DbError> {
    recover(path)?;
    conversion.check_source(path, FileFormat::detect(path)?)?;
    require_cipher().await?;
    let files = SwapFiles::of(path);
    let expected = match verified_copy(path, &files.copy, conversion).await {
        Ok(expected) => expected,
        Err(error) => {
            remove_if_present(&files.copy)?;
            return Err(error);
        }
    };
    files.swap()?;
    match verify(path, conversion.target_key(), &expected).await {
        Ok(()) => files.discard_backup(),
        Err(error) => {
            files.restore()?;
            Err(error)
        }
    }
}

/// Finishes or undoes a conversion that a crash interrupted at `path`.
///
/// A copy is only swapped in after it was verified and synced, and each
/// rename is atomic, so: a backup beside the database means the swap
/// finished; a backup without the database means it stopped halfway.
pub fn recover(path: &Path) -> Result<Recovery, DbError> {
    let files = SwapFiles::of(path);
    if files.backup.exists() {
        if path.exists() {
            files.discard_backup()?;
            return Ok(Recovery::DiscardedBackup);
        }
        files.restore()?;
        remove_if_present(&files.copy)?;
        return Ok(Recovery::RestoredOriginal);
    }
    if files.copy.exists() {
        remove_if_present(&files.copy)?;
        return Ok(Recovery::DiscardedCopy);
    }
    Ok(Recovery::Clean)
}

/// Writes and verifies the converted copy of `source` at `copy`, returning
/// the original's fingerprint.
async fn verified_copy(
    source: &Path,
    copy: &Path,
    conversion: Conversion<'_>,
) -> Result<Fingerprint, DbError> {
    let expected = export(source, copy, conversion).await?;
    verify(copy, conversion.target_key(), &expected).await?;
    Ok(expected)
}

/// Writes the converted copy of `source` to `copy` and returns the
/// original's fingerprint.
async fn export(
    source: &Path,
    copy: &Path,
    conversion: Conversion<'_>,
) -> Result<Fingerprint, DbError> {
    let key = conversion.source_key();
    let mut connection = open(source, key).await?;
    let expected = Fingerprint::of(&mut connection).await?;
    let copy_key = conversion.target_key().map(DbKey::bound_value);
    sqlx::query(&format!("ATTACH DATABASE ?1 AS {TARGET_SCHEMA} KEY ?2"))
        .bind(copy.to_string_lossy().into_owned())
        .bind(copy_key.as_deref().map_or("", String::as_str))
        .execute(&mut connection)
        .await?;
    let version: i64 = sqlx::query_scalar("PRAGMA main.user_version")
        .fetch_one(&mut connection)
        .await?;
    // The export copies schema and rows; the header fields are set by hand.
    // WAL is the app's mode, and a read-only reader cannot switch to it.
    let steps = [
        format!("SELECT sqlcipher_export('{TARGET_SCHEMA}')"),
        format!("PRAGMA {TARGET_SCHEMA}.user_version = {version}"),
        format!("PRAGMA {TARGET_SCHEMA}.journal_mode = WAL"),
        format!("DETACH DATABASE {TARGET_SCHEMA}"),
    ];
    for step in &steps {
        sqlx::query(step).execute(&mut connection).await?;
    }
    connection.close().await?;
    ensure_closed(source)?;
    Ok(expected)
}

/// Fails unless the file at `path` opens with `key`, passes SQLite's
/// `quick_check` and matches `expected` table for table.
async fn verify(path: &Path, key: Option<&DbKey>, expected: &Fingerprint) -> Result<(), DbError> {
    let mut connection = open(path, key).await?;
    let check: String = sqlx::query_scalar("PRAGMA quick_check")
        .fetch_one(&mut connection)
        .await?;
    let actual = Fingerprint::of(&mut connection).await?;
    connection.close().await?;
    if check == "ok" && actual == *expected {
        Ok(())
    } else {
        log::error!(target: "db", "converted database failed verification ({check})");
        Err(DbError::Mismatch)
    }
}

/// One connection to an existing file, keyed when `key` is given.
///
/// It may create files: an `ATTACH` inherits the connection's open flags,
/// and the copy does not exist yet. Callers check the file exists first.
async fn open(path: &Path, key: Option<&DbKey>) -> Result<SqliteConnection, DbError> {
    if !path.is_file() {
        return Err(DbError::Missing(path.to_path_buf()));
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .disable_statement_logging();
    let mut connection = keyed(options, key)
        .connect()
        .await
        .map_err(|error| open_error(error, key))?;
    // SQLCipher reads the first page lazily: probe it so a wrong key fails here.
    sqlx::query("SELECT count(*) FROM sqlite_schema")
        .execute(&mut connection)
        .await
        .map_err(|error| open_error(error, key))?;
    Ok(connection)
}

/// Fails with [`DbError::InUse`] when `path` still has a write-ahead log:
/// SQLite removes it when the last connection closes, so another process
/// (an older app instance) has the file open and could write to it.
fn ensure_closed(path: &Path) -> Result<(), DbError> {
    if side_file(path, "-wal").exists() {
        return Err(DbError::InUse);
    }
    Ok(())
}

/// The three paths a conversion moves files between.
struct SwapFiles {
    /// The live database.
    live: PathBuf,
    /// The converted copy before it is swapped in.
    copy: PathBuf,
    /// The original while the swapped-in copy is verified.
    backup: PathBuf,
}

impl SwapFiles {
    fn of(path: &Path) -> Self {
        Self {
            live: path.to_path_buf(),
            copy: side_file(path, ".converting"),
            backup: side_file(path, ".backup"),
        }
    }

    /// Original → backup, then copy → live, then syncs the folder.
    fn swap(&self) -> Result<(), DbError> {
        remove_if_present(&side_file(&self.live, "-shm"))?;
        std::fs::rename(&self.live, &self.backup).map_err(DbError::Swap)?;
        std::fs::rename(&self.copy, &self.live).map_err(DbError::Swap)?;
        sync_folder(&self.live)
    }

    /// Backup → live, replacing whatever is there.
    fn restore(&self) -> Result<(), DbError> {
        std::fs::rename(&self.backup, &self.live).map_err(DbError::Swap)?;
        sync_folder(&self.live)
    }

    fn discard_backup(&self) -> Result<(), DbError> {
        remove_if_present(&self.backup)?;
        sync_folder(&self.live)
    }
}

/// `path` with `suffix` appended to its file name.
fn side_file(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

fn remove_if_present(path: &Path) -> Result<(), DbError> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(DbError::Swap(error)),
        _ => Ok(()),
    }
}

/// Flushes the folder holding `path`, so the renames survive a power loss.
fn sync_folder(path: &Path) -> Result<(), DbError> {
    let Some(folder) = path.parent() else {
        return Ok(());
    };
    std::fs::File::open(folder)
        .and_then(|folder| folder.sync_all())
        .map_err(DbError::Swap)
}

#[cfg(test)]
#[path = "convert_tests.rs"]
mod tests;
