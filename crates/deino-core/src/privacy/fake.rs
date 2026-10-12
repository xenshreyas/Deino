//! Fakes for the startup conversion: a database file and a key store that
//! write every step into one shared [`Journal`], so tests can assert the
//! order of Keychain and file operations.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use deino_db::{Conversion, DbError, FileFormat, Recovery};
use deino_secrets::{DatabaseKeyStore, InMemoryDatabaseKeyStore, SecretError, SecretValue};

use super::{DatabaseFile, Direction};

/// One operation the fakes saw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// `recover` ran.
    Recover,
    /// A key was saved.
    SaveKey,
    /// The key was forgotten.
    ForgetKey,
    /// A conversion ran (and succeeded unless the file was told to fail).
    Convert(Direction),
}

/// The steps both fakes saw, in order.
#[derive(Clone, Debug, Default)]
pub struct Journal(Arc<Mutex<Vec<Step>>>);

impl Journal {
    fn push(&self, step: Step) {
        lock(&self.0).push(step);
    }

    /// Every step so far, without the `Recover` noise.
    pub fn steps(&self) -> Vec<Step> {
        lock(&self.0)
            .iter()
            .copied()
            .filter(|step| *step != Step::Recover)
            .collect()
    }

    /// True when `recover` ran at least once.
    pub fn recovered(&self) -> bool {
        lock(&self.0).contains(&Step::Recover)
    }
}

/// A database file that is only a format.
#[derive(Debug)]
pub struct FakeDatabaseFile {
    format: Mutex<FileFormat>,
    fail_convert: bool,
    journal: Journal,
}

impl FakeDatabaseFile {
    /// A file in `format` whose conversions succeed.
    pub fn new(format: FileFormat, journal: Journal) -> Self {
        Self {
            format: Mutex::new(format),
            fail_convert: false,
            journal,
        }
    }

    /// The same file, whose conversions fail and leave it unchanged.
    #[must_use]
    pub fn failing(self) -> Self {
        Self {
            fail_convert: true,
            ..self
        }
    }

    /// The format now.
    pub fn current(&self) -> FileFormat {
        *lock(&self.format)
    }
}

#[async_trait]
impl DatabaseFile for FakeDatabaseFile {
    fn recover(&self) -> Result<Recovery, DbError> {
        self.journal.push(Step::Recover);
        Ok(Recovery::Clean)
    }

    fn format(&self) -> Result<FileFormat, DbError> {
        Ok(self.current())
    }

    async fn convert(&self, conversion: Conversion<'_>) -> Result<(), DbError> {
        let (direction, after) = match conversion {
            Conversion::Encrypt(_) => (Direction::Encrypt, FileFormat::Encrypted),
            Conversion::Decrypt(_) => (Direction::Decrypt, FileFormat::Plaintext),
        };
        self.journal.push(Step::Convert(direction));
        if self.fail_convert {
            return Err(DbError::Mismatch);
        }
        *lock(&self.format) = after;
        Ok(())
    }
}

/// An in-memory key store that journals saves and forgets.
#[derive(Debug, Default)]
pub struct JournalKeyStore {
    inner: InMemoryDatabaseKeyStore,
    journal: Journal,
    refuse_saves: bool,
}

impl JournalKeyStore {
    /// An empty store writing into `journal`.
    pub fn new(journal: Journal) -> Self {
        Self {
            inner: InMemoryDatabaseKeyStore::default(),
            journal,
            refuse_saves: false,
        }
    }

    /// The same store, whose saves fail as a refusing Keychain's would.
    #[must_use]
    pub fn refusing_saves(self) -> Self {
        Self {
            refuse_saves: true,
            ..self
        }
    }

    /// The store under the journal, to seed a key or deny reads.
    pub fn inner(&self) -> &InMemoryDatabaseKeyStore {
        &self.inner
    }
}

#[async_trait]
impl DatabaseKeyStore for JournalKeyStore {
    async fn save_database_key(&self, key: SecretValue) -> Result<(), SecretError> {
        self.journal.push(Step::SaveKey);
        if self.refuse_saves {
            return Err(SecretError::DatabaseKeyDenied);
        }
        self.inner.save_database_key(key).await
    }

    async fn load_database_key(&self) -> Result<Option<SecretValue>, SecretError> {
        self.inner.load_database_key().await
    }

    async fn forget_database_key(&self) -> Result<(), SecretError> {
        self.journal.push(Step::ForgetKey);
        self.inner.forget_database_key().await
    }
}

/// A panic in another test thread cannot leave the journal half-written.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
