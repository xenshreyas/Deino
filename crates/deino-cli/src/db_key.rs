//! The key of an encrypted database (PRV-02, ADR 0038).
//!
//! The app keeps the key in one Keychain item under its bundle identifier.
//! The CLI reads that item only when the file on disk is encrypted, so a
//! plaintext database never touches the Keychain. macOS may ask the user to
//! let `deino` read the item the first time ("Always Allow" keeps it quiet).

use std::path::Path;

use deino_db::{DbError, DbKey, FileFormat};
use deino_secrets::DatabaseKeyStore;

use crate::CliError;

/// The key that opens the database at `path`: `None` for a plaintext file,
/// the saved key for an encrypted one, and an error naming the fix when the
/// key is missing or unreadable.
pub async fn key_for(path: &Path, store: &dyn DatabaseKeyStore) -> Result<Option<DbKey>, CliError> {
    if FileFormat::detect(path)? != FileFormat::Encrypted {
        return Ok(None);
    }
    let saved = store
        .load_database_key()
        .await
        .map_err(CliError::Keychain)?
        .ok_or(CliError::Database(DbError::KeyRequired))?;
    DbKey::from_hex(saved.expose())
        .map(Some)
        .map_err(|_| CliError::Database(DbError::WrongKey))
}

#[cfg(test)]
mod tests {
    use deino_db::{Conversion, Database, DbLocation, MIGRATOR, convert};
    use deino_secrets::{InMemoryDatabaseKeyStore, SecretValue};

    use super::*;

    async fn plaintext_file(dir: &Path) -> std::path::PathBuf {
        let path = dir.join("deino.sqlite");
        let database = Database::open(&DbLocation::File(path.clone()), &MIGRATOR)
            .await
            .unwrap();
        database.close().await;
        path
    }

    #[tokio::test]
    async fn a_plaintext_database_never_reads_the_keychain() {
        let dir = tempfile::tempdir().unwrap();
        let path = plaintext_file(dir.path()).await;
        let store = InMemoryDatabaseKeyStore::default();
        store
            .save_database_key(SecretValue::parse(&"0f".repeat(32)).unwrap())
            .await
            .unwrap();
        store.deny();

        assert!(key_for(&path, &store).await.unwrap().is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_encrypted_database_needs_the_saved_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = plaintext_file(dir.path()).await;
        let key = DbKey::generate().unwrap();
        convert(&path, Conversion::Encrypt(&key)).await.unwrap();
        let store = InMemoryDatabaseKeyStore::default();

        let missing = key_for(&path, &store).await.map(|_| ()).unwrap_err();
        assert!(missing.to_string().contains("Keychain"), "{missing}");
        assert_eq!(missing.exit_code(), 5);

        let saved = SecretValue::parse(&key.to_hex()).unwrap();
        store.save_database_key(saved).await.unwrap();
        let found = key_for(&path, &store).await.unwrap().unwrap();
        assert!(found == key);
        let opened = Database::open_read_only_keyed(&path, Some(&found), &MIGRATOR).await;
        assert!(opened.is_ok());

        store.deny();
        let denied = key_for(&path, &store).await.map(|_| ()).unwrap_err();
        assert_eq!(denied.exit_code(), 5, "{denied}");
    }
}
