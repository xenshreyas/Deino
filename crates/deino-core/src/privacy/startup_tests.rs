//! The startup conversion's state machine over fakes: which conversion runs
//! for each file and setting, and that the Keychain is written before an
//! encryption and cleared only after a decryption.

use deino_db::{DbKey, FileFormat};
use deino_secrets::SecretValue;

use super::fake::{FakeDatabaseFile, Journal, JournalKeyStore, Step};
use super::{Direction, PrivacyError, StartupConversion, StartupOutcome};

struct Harness {
    journal: Journal,
    file: FakeDatabaseFile,
    keys: JournalKeyStore,
}

impl Harness {
    fn new(format: FileFormat) -> Self {
        let journal = Journal::default();
        Self {
            file: FakeDatabaseFile::new(format, journal.clone()),
            keys: JournalKeyStore::new(journal.clone()),
            journal,
        }
    }

    fn failing(self) -> Self {
        Self {
            file: self.file.failing(),
            ..self
        }
    }

    async fn with_saved_key(self) -> Self {
        let text = DbKey::generate().unwrap().to_hex();
        self.keys
            .inner()
            .save_database_key(SecretValue::parse(&text).unwrap())
            .await
            .unwrap();
        self
    }

    fn conversion(&self) -> StartupConversion<'_> {
        StartupConversion::new(&self.file, &self.keys)
    }

    fn saved_key(&self) -> Option<String> {
        self.keys.inner().stored()
    }
}

use deino_secrets::DatabaseKeyStore as _;

#[tokio::test]
async fn a_plaintext_file_with_encryption_off_opens_as_it_is() {
    let harness = Harness::new(FileFormat::Plaintext);

    let prepared = harness.conversion().prepare(false).await.unwrap();

    assert!(prepared.key.is_none());
    assert_eq!(prepared.outcome, StartupOutcome::Unchanged);
    assert!(harness.journal.recovered(), "recover runs first");
    assert!(harness.journal.steps().is_empty());
}

#[tokio::test]
async fn encrypting_saves_the_key_before_converting() {
    let harness = Harness::new(FileFormat::Plaintext);

    let prepared = harness.conversion().prepare(true).await.unwrap();

    assert_eq!(
        harness.journal.steps(),
        [Step::SaveKey, Step::Convert(Direction::Encrypt)]
    );
    assert_eq!(harness.file.current(), FileFormat::Encrypted);
    let saved = harness.saved_key().unwrap();
    assert!(
        prepared.key.unwrap().to_hex().as_str() == saved,
        "opens with the saved key"
    );
    assert_eq!(
        prepared.outcome,
        StartupOutcome::Converted {
            direction: Direction::Encrypt
        }
    );
}

#[tokio::test]
async fn a_failed_encryption_opens_the_unchanged_plaintext_file() {
    let harness = Harness::new(FileFormat::Plaintext).failing();

    let prepared = harness.conversion().prepare(true).await.unwrap();

    assert!(prepared.key.is_none());
    assert_eq!(harness.file.current(), FileFormat::Plaintext);
    assert_eq!(
        prepared.outcome,
        StartupOutcome::Failed {
            direction: Direction::Encrypt
        }
    );
}

#[tokio::test]
async fn an_encryption_whose_key_cannot_be_saved_never_converts() {
    let mut harness = Harness::new(FileFormat::Plaintext);
    harness.keys = JournalKeyStore::new(harness.journal.clone()).refusing_saves();

    let prepared = harness.conversion().prepare(true).await.unwrap();

    assert_eq!(harness.journal.steps(), [Step::SaveKey]);
    assert!(prepared.key.is_none());
    assert_eq!(harness.file.current(), FileFormat::Plaintext);
    assert_eq!(
        prepared.outcome,
        StartupOutcome::Failed {
            direction: Direction::Encrypt
        }
    );
}

#[tokio::test]
async fn decrypting_forgets_the_key_only_after_converting() {
    let harness = Harness::new(FileFormat::Encrypted).with_saved_key().await;

    let prepared = harness.conversion().prepare(false).await.unwrap();

    assert_eq!(
        harness.journal.steps(),
        [Step::Convert(Direction::Decrypt), Step::ForgetKey]
    );
    assert!(prepared.key.is_none());
    assert!(harness.saved_key().is_none());
    assert_eq!(harness.file.current(), FileFormat::Plaintext);
}

#[tokio::test]
async fn a_failed_decryption_keeps_the_key_and_opens_encrypted() {
    let harness = Harness::new(FileFormat::Encrypted)
        .with_saved_key()
        .await
        .failing();

    let prepared = harness.conversion().prepare(false).await.unwrap();

    assert!(prepared.is_encrypted());
    assert!(
        harness.saved_key().is_some(),
        "the key is never forgotten early"
    );
    assert!(!harness.journal.steps().contains(&Step::ForgetKey));
    assert_eq!(
        prepared.outcome,
        StartupOutcome::Failed {
            direction: Direction::Decrypt
        }
    );
}

#[tokio::test]
async fn an_encrypted_file_opens_with_its_saved_key() {
    let harness = Harness::new(FileFormat::Encrypted).with_saved_key().await;

    let prepared = harness.conversion().prepare(true).await.unwrap();

    assert_eq!(
        prepared.key.unwrap().to_hex().as_str(),
        harness.saved_key().unwrap()
    );
    assert_eq!(prepared.outcome, StartupOutcome::Unchanged);
}

#[tokio::test]
async fn an_encrypted_file_without_a_key_is_a_typed_error() {
    for encrypt in [true, false] {
        let harness = Harness::new(FileFormat::Encrypted);

        let error = harness.conversion().prepare(encrypt).await.err().unwrap();

        assert!(matches!(error, PrivacyError::KeyMissing));
        assert!(
            !harness
                .journal
                .steps()
                .contains(&Step::Convert(Direction::Decrypt))
        );
    }
}

#[tokio::test]
async fn a_denied_keychain_is_a_typed_error() {
    let harness = Harness::new(FileFormat::Encrypted).with_saved_key().await;
    harness.keys.inner().deny();

    let error = harness.conversion().prepare(true).await.err().unwrap();

    assert!(matches!(error, PrivacyError::KeyUnreadable(_)));
}

#[tokio::test]
async fn a_new_install_with_encryption_on_is_created_encrypted() {
    let harness = Harness::new(FileFormat::Missing);

    let prepared = harness.conversion().prepare(true).await.unwrap();

    assert_eq!(harness.journal.steps(), [Step::SaveKey]);
    assert!(prepared.is_encrypted());
}

#[tokio::test]
async fn pending_names_the_conversion_without_reading_the_key() {
    let cases = [
        (FileFormat::Plaintext, true, Some(Direction::Encrypt)),
        (FileFormat::Encrypted, false, Some(Direction::Decrypt)),
        (FileFormat::Plaintext, false, None),
        (FileFormat::Encrypted, true, None),
        (FileFormat::Missing, true, None),
    ];
    for (format, encrypt, expected) in cases {
        let harness = Harness::new(format);
        harness.keys.inner().deny();

        let pending = harness.conversion().pending(encrypt).unwrap();

        assert_eq!(pending, expected, "{format:?} with encryption {encrypt}");
        assert!(harness.journal.recovered());
    }
}
