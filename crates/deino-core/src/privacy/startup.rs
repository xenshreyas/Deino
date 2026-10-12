//! [`StartupConversion`]: the encryption state machine that runs before the
//! database opens.

use deino_db::{Conversion, DbKey, FileFormat, Recovery};
use deino_secrets::{DatabaseKeyStore, SecretValue};
use serde::Serialize;

use super::{DatabaseFile, PrivacyError};

/// Which way a conversion goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Plaintext → encrypted.
    Encrypt,
    /// Encrypted → plaintext.
    Decrypt,
}

/// What startup did to the database file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StartupOutcome {
    /// The file already matched the setting.
    Unchanged,
    /// The file was converted and verified.
    Converted {
        /// Which way.
        direction: Direction,
    },
    /// The conversion failed; the file is unchanged and opens as it was.
    Failed {
        /// Which way it was going.
        direction: Direction,
    },
}

/// The key the database opens with, and how it got there.
pub struct PreparedDatabase {
    /// `Some` for an encrypted file (or a new one created encrypted).
    pub key: Option<DbKey>,
    /// What startup did.
    pub outcome: StartupOutcome,
}

impl PreparedDatabase {
    fn plaintext(outcome: StartupOutcome) -> Self {
        Self { key: None, outcome }
    }

    fn keyed(key: DbKey, outcome: StartupOutcome) -> Self {
        Self {
            key: Some(key),
            outcome,
        }
    }

    /// True when the database will be encrypted once open.
    pub fn is_encrypted(&self) -> bool {
        self.key.is_some()
    }
}

/// Brings the closed database file in line with the encryption setting.
pub struct StartupConversion<'a> {
    file: &'a dyn DatabaseFile,
    keys: &'a dyn DatabaseKeyStore,
}

impl<'a> StartupConversion<'a> {
    /// Works on `file` with the key in `keys`.
    pub fn new(file: &'a dyn DatabaseFile, keys: &'a dyn DatabaseKeyStore) -> Self {
        Self { file, keys }
    }

    /// Recovers an interrupted conversion and says which conversion
    /// `encrypt` calls for, without reading the key: the app shows a
    /// progress screen while it runs.
    pub fn pending(&self, encrypt: bool) -> Result<Option<Direction>, PrivacyError> {
        self.recover()?;
        Ok(match (self.file.format()?, encrypt) {
            (FileFormat::Plaintext, true) => Some(Direction::Encrypt),
            (FileFormat::Encrypted, false) => Some(Direction::Decrypt),
            _ => None,
        })
    }

    /// Recovers, converts when the file and `encrypt` disagree, and returns
    /// the key to open the database with.
    pub async fn prepare(&self, encrypt: bool) -> Result<PreparedDatabase, PrivacyError> {
        self.recover()?;
        match (self.file.format()?, encrypt) {
            (FileFormat::Missing | FileFormat::Plaintext, false) => {
                Ok(PreparedDatabase::plaintext(StartupOutcome::Unchanged))
            }
            (FileFormat::Missing, true) => Ok(self.create_encrypted().await),
            (FileFormat::Plaintext, true) => Ok(self.encrypt().await),
            (FileFormat::Encrypted, true) => {
                let key = self.saved_key().await?;
                Ok(PreparedDatabase::keyed(key, StartupOutcome::Unchanged))
            }
            (FileFormat::Encrypted, false) => self.decrypt().await,
        }
    }

    fn recover(&self) -> Result<(), PrivacyError> {
        let recovery = self.file.recover()?;
        if recovery != Recovery::Clean {
            log::info!(target: "privacy", "finished an interrupted conversion: {recovery:?}");
        }
        Ok(())
    }

    /// A new install with encryption on: SQLCipher creates the file
    /// encrypted on first open, once the key is saved.
    async fn create_encrypted(&self) -> PreparedDatabase {
        match self.new_saved_key().await {
            Ok(key) => PreparedDatabase::keyed(key, StartupOutcome::Unchanged),
            Err(error) => failed(Direction::Encrypt, &error),
        }
    }

    /// Saves a new key, then encrypts: the file is never encrypted with a
    /// key the Keychain does not hold.
    async fn encrypt(&self) -> PreparedDatabase {
        let key = match self.new_saved_key().await {
            Ok(key) => key,
            Err(error) => return failed(Direction::Encrypt, &error),
        };
        match self.file.convert(Conversion::Encrypt(&key)).await {
            Ok(()) => PreparedDatabase::keyed(key, converted(Direction::Encrypt)),
            Err(error) => failed(Direction::Encrypt, &error),
        }
    }

    /// Decrypts, then forgets the key: the key outlives the encrypted file.
    async fn decrypt(&self) -> Result<PreparedDatabase, PrivacyError> {
        let key = self.saved_key().await?;
        if let Err(error) = self.file.convert(Conversion::Decrypt(&key)).await {
            log::warn!(target: "privacy", "could not decrypt the database: {error}");
            let outcome = StartupOutcome::Failed {
                direction: Direction::Decrypt,
            };
            return Ok(PreparedDatabase::keyed(key, outcome));
        }
        // The file is plaintext and verified; a key left behind is harmless.
        if let Err(error) = self.keys.forget_database_key().await {
            log::warn!(target: "privacy", "could not forget the database key: {error}");
        }
        Ok(PreparedDatabase::plaintext(converted(Direction::Decrypt)))
    }

    async fn new_saved_key(&self) -> Result<DbKey, PrivacyError> {
        let key = DbKey::generate().map_err(|_| PrivacyError::KeyMalformed)?;
        let text = SecretValue::parse(&key.to_hex()).map_err(|_| PrivacyError::KeyMalformed)?;
        self.keys
            .save_database_key(text)
            .await
            .map_err(PrivacyError::KeyUnreadable)?;
        Ok(key)
    }

    async fn saved_key(&self) -> Result<DbKey, PrivacyError> {
        let saved = self
            .keys
            .load_database_key()
            .await
            .map_err(PrivacyError::KeyUnreadable)?
            .ok_or(PrivacyError::KeyMissing)?;
        DbKey::from_hex(saved.expose()).map_err(|_| PrivacyError::KeyMalformed)
    }
}

fn converted(direction: Direction) -> StartupOutcome {
    StartupOutcome::Converted { direction }
}

/// The file stays plaintext: the conversion (or saving its key) failed.
fn failed(direction: Direction, error: &dyn std::error::Error) -> PreparedDatabase {
    log::warn!(target: "privacy", "could not {direction:?} the database: {error}");
    PreparedDatabase::plaintext(StartupOutcome::Failed { direction })
}
