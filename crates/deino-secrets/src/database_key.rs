//! [`DatabaseKeyStore`]: the one Keychain item that holds the database
//! encryption key (PRV-02, ADR 0038).
//!
//! The key is kept as its 64-hex-digit text form in a [`SecretValue`], so it
//! is never printed and is wiped from memory on drop. This crate does not
//! know the key's shape: `deino_db::DbKey` parses and writes the text. The
//! UI never reads the key; only the app (to open and convert the database)
//! and the `deino` CLI (to read it) do.

use async_trait::async_trait;

use crate::{SecretError, SecretValue};

/// The keychain account of the database key.
pub const DATABASE_KEY_ACCOUNT: &str = "database.encryption-key";

/// Where the database key lives.
///
/// Implementations honour the [`SecretStore`](crate::SecretStore) contract:
/// `save` replaces, `load` of a missing key is `Ok(None)`, `forget` of a
/// missing key succeeds, and a key the OS refuses to hand over fails with
/// [`SecretError::DatabaseKeyDenied`].
#[async_trait]
pub trait DatabaseKeyStore: Send + Sync {
    /// Saves the key's text form, replacing any saved key.
    async fn save_database_key(&self, key: SecretValue) -> Result<(), SecretError>;

    /// The saved key's text form, or `None` when none is saved.
    async fn load_database_key(&self) -> Result<Option<SecretValue>, SecretError>;

    /// Forgets the key; succeeds when none is saved. Only call it once the
    /// database is plaintext again, or the database is lost.
    async fn forget_database_key(&self) -> Result<(), SecretError>;
}
