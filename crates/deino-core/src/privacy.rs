//! Encryption at rest at startup (PRV-02, ADR 0041).
//!
//! Before the app opens `deino.sqlite`, [`StartupConversion`] brings the
//! file in line with the `privacy.encryptDatabase` setting: it finishes or
//! undoes a conversion a crash interrupted (`deino_db::recover`), then
//! encrypts a plaintext file or decrypts an encrypted one, and hands back
//! the key the database opens with.
//!
//! Two rules keep the library safe:
//!
//! - the key reaches the Keychain **before** the file is encrypted, so an
//!   encrypted file always has its key saved;
//! - the key is forgotten only **after** the decrypted file is verified and
//!   swapped in (`deino_db::convert` verifies it).
//!
//! A conversion that fails leaves the file as it was (`convert` keeps the
//! original until the copy is verified), so startup opens the file in its
//! current format and reports [`StartupOutcome::Failed`] instead of
//! blocking the app. Only an encrypted file whose key cannot be read is an
//! error ([`PrivacyError`]): the app then shows a calm blocking screen and
//! never deletes anything.
//!
//! The file is reached through the [`DatabaseFile`] port and the key through
//! `deino_secrets::DatabaseKeyStore`, so the rules are unit-tested with
//! fakes.

mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod file;
mod startup;
#[cfg(test)]
mod startup_tests;

pub use error::PrivacyError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeDatabaseFile, Journal, JournalKeyStore, Step};
pub use file::{DatabaseFile, OnDiskDatabase};
pub use startup::{Direction, PreparedDatabase, StartupConversion, StartupOutcome};
