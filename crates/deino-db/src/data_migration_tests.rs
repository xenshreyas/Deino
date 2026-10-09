//! Data migrations: when a Rust step runs, what schema it sees, and the
//! FND-06 drop that must leave no plaintext key on disk.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;

use super::*;

/// `20261009061225_fnd06_drop_legacy_settings.sql`.
const DROP_LEGACY_SETTINGS: i64 = 20_261_009_061_225;
/// A key that must not survive the drop anywhere in the database file.
const CANARY_KEY: &str = "sk-canary-0123456789abcdef";

/// Counts its runs and checks that `settings` still exists when it runs.
#[derive(Default)]
struct SeesLegacySettings {
    runs: AtomicUsize,
}

#[async_trait]
impl DataMigration for SeesLegacySettings {
    fn runs_before(&self) -> i64 {
        DROP_LEGACY_SETTINGS
    }

    async fn run(&self, database: &Database) -> Result<(), DataMigrationError> {
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings")
            .fetch_one(database.reader())
            .await?;
        assert_eq!(rows, 0);
        self.runs.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// Always fails, like a step whose destination is unavailable.
struct Fails;

#[async_trait]
impl DataMigration for Fails {
    fn runs_before(&self) -> i64 {
        DROP_LEGACY_SETTINGS
    }

    async fn run(&self, _database: &Database) -> Result<(), DataMigrationError> {
        Err("the keychain is locked".into())
    }
}

async fn has_table(database: &Database, name: &str) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE name = ?1)")
        .bind(name)
        .fetch_one(database.reader())
        .await
        .unwrap()
}

#[tokio::test]
async fn a_step_runs_against_the_schema_just_before_its_migration() {
    let step = SeesLegacySettings::default();

    let database = Database::open_with(&DbLocation::Memory, &MIGRATOR, &[&step])
        .await
        .unwrap();

    assert_eq!(step.runs.load(Ordering::SeqCst), 1);
    assert!(!has_table(&database, "settings").await);
    assert!(has_table(&database, "meetings").await);
}

#[tokio::test]
async fn a_step_whose_migration_is_applied_never_runs_again() {
    let directory = tempfile::tempdir().unwrap();
    let location = DbLocation::File(directory.path().join("deino.sqlite"));
    Database::open(&location, &MIGRATOR)
        .await
        .unwrap()
        .close()
        .await;
    let step = SeesLegacySettings::default();

    Database::open_with(&location, &MIGRATOR, &[&step])
        .await
        .unwrap();

    assert_eq!(step.runs.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_failed_step_keeps_its_data_and_runs_again_on_the_next_open() {
    let directory = tempfile::tempdir().unwrap();
    let location = DbLocation::File(directory.path().join("deino.sqlite"));

    let error = Database::open_with(&location, &MIGRATOR, &[&Fails])
        .await
        .unwrap_err();
    assert!(matches!(error, DbError::DataMigration(_)), "{error:?}");
    let step = SeesLegacySettings::default();
    let database = Database::open_with(&location, &MIGRATOR, &[&step])
        .await
        .unwrap();

    assert_eq!(step.runs.load(Ordering::SeqCst), 1);
    assert!(!has_table(&database, "settings").await);
}

#[tokio::test]
async fn dropping_the_legacy_settings_leaves_no_key_bytes_in_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("deino.sqlite");
    let location = DbLocation::File(path.clone());
    let legacy = Database::connect(&location).await.unwrap();
    legacy
        .migrate_before(&MIGRATOR, DROP_LEGACY_SETTINGS)
        .await
        .unwrap();
    sqlx::query("INSERT INTO settings (id, provider, model, whisperModel, openaiApiKey) VALUES ('1', 'openai', 'gpt-4o', 'large-v3', ?1)")
        .bind(CANARY_KEY)
        .execute(legacy.writer())
        .await
        .unwrap();
    legacy.close().await;
    assert!(
        file_contains(&path, CANARY_KEY),
        "the fixture wrote the key"
    );

    Database::open(&location, &MIGRATOR)
        .await
        .unwrap()
        .close()
        .await;

    assert!(!file_contains(&path, CANARY_KEY));
}

/// True when `needle` appears in the database file or its WAL.
fn file_contains(path: &Path, needle: &str) -> bool {
    let mut wal = path.as_os_str().to_owned();
    wal.push("-wal");
    [path.to_path_buf(), wal.into()].iter().any(|file| {
        std::fs::read(file).is_ok_and(|bytes| {
            bytes
                .windows(needle.len())
                .any(|window| window == needle.as_bytes())
        })
    })
}
