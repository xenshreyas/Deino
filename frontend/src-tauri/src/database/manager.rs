//! The legacy handle to Deino's database.
//!
//! Opens `deino.sqlite` in the app data dir through [`deino_db::Database`]
//! (WAL, one writer, a read pool, every migration in `crates/deino-db`).
//! Inherited commands reach SQLite through [`DatabaseManager::pool`], which is
//! the writer connection: its commit hooks report every legacy write to the
//! UI as `DbChanged`, so the new TanStack Query caches stay fresh even when
//! legacy code writes. New code receives the `Database` from the composition
//! root instead (`composition.rs`).
//!
//! Before the file opens, `deino_core::privacy::StartupConversion` brings it
//! in line with `privacy.encryptDatabase` (PRV-02, ADR 0041): it recovers an
//! interrupted conversion, encrypts or decrypts, and hands back the key the
//! database opens with. No connection is open while it runs.

use std::fs;
use std::path::{Path, PathBuf};

use deino_core::privacy::{
    Direction, OnDiskDatabase, PreparedDatabase, PrivacyError, StartupConversion, StartupOutcome,
};
use deino_db::{DataMigration, Database, DbError, DbKey, DbLocation, MIGRATOR};
use deino_secrets::KeychainSecretStore;
use sqlx::SqlitePool;
use tauri::Manager;

/// File name of Deino's SQLite database inside the app data dir.
pub const DB_FILE_NAME: &str = "deino.sqlite";

/// Suffixes of the SQLite side files that can be orphaned by a crash in WAL mode.
const WAL_SIDE_FILE_SUFFIXES: [&str; 2] = ["-wal", "-shm"];

/// Why the database did not open.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// The file could not be brought in line with the encryption setting,
    /// or its key could not be read.
    #[error(transparent)]
    Privacy(#[from] PrivacyError),
    /// SQLite could not open the file (a wrong key included).
    #[error(transparent)]
    Database(#[from] DbError),
}

/// Whether the open database is encrypted, and what startup did to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncryptionReport {
    /// True when the file on disk is encrypted.
    pub encrypted: bool,
    /// What the startup conversion did.
    pub outcome: StartupOutcome,
}

/// Shared handle to the application's database.
#[derive(Clone, Debug)]
pub struct DatabaseManager {
    database: Database,
    encryption: EncryptionReport,
}

impl DatabaseManager {
    /// Opens (creating it if needed) the database at `db_path` with `key`,
    /// runs all migrations and, between them, the Rust data migrations in
    /// `steps`.
    async fn open(
        db_path: &Path,
        key: Option<&DbKey>,
        steps: &[&dyn DataMigration],
    ) -> Result<Database, DbError> {
        let location = DbLocation::File(db_path.to_path_buf());
        Database::open_keyed(&location, key, &MIGRATOR, steps).await
    }

    /// Converts the file if the encryption setting asks for it, then opens
    /// it, retrying once after removing orphaned WAL side files when SQLite
    /// reports corruption. The legacy SQLite settings move to the settings
    /// file and the Keychain on the way, and the legacy summaries become
    /// enhanced notes (NOT-02).
    pub async fn new_from_app_handle(app_handle: &tauri::AppHandle) -> Result<Self, OpenError> {
        let db_path = Self::db_path(app_handle);
        log::info!("Database path: {}", db_path.display());
        let prepared = Self::prepare_file(app_handle, &db_path).await?;
        let import = crate::composition::legacy_settings_import(app_handle);
        let summaries = deino_core::enhance::LegacySummaryImport;
        let steps: Vec<&dyn DataMigration> = import
            .as_ref()
            .map(|step| step as &dyn DataMigration)
            .into_iter()
            .chain([&summaries as &dyn DataMigration])
            .collect();
        let key = prepared.key.as_ref();
        let database = match Self::open(&db_path, key, &steps).await {
            Ok(database) => database,
            Err(e) if e.is_corruption() => {
                log::warn!("Database appears corrupted, likely an orphaned WAL file: {:?}", e);
                Self::remove_wal_side_files(&db_path);
                Self::open(&db_path, key, &steps).await?
            }
            Err(e) => {
                log::error!("Database connection failed: {:?}", e);
                return Err(e.into());
            }
        };
        let encryption = EncryptionReport {
            encrypted: prepared.is_encrypted(),
            outcome: prepared.outcome,
        };
        Ok(Self { database, encryption })
    }

    /// The conversion the next open will run, if any, without reading the
    /// key: startup shows a progress screen while it runs.
    pub async fn pending_conversion(
        app_handle: &tauri::AppHandle,
    ) -> Result<Option<Direction>, PrivacyError> {
        let encrypt = Self::encryption_wanted(app_handle).await;
        let keys = Self::key_store(app_handle);
        let file = OnDiskDatabase::new(Self::db_path(app_handle));
        StartupConversion::new(&file, &keys).pending(encrypt)
    }

    /// Recovers, converts and returns the key the file opens with.
    async fn prepare_file(
        app_handle: &tauri::AppHandle,
        db_path: &Path,
    ) -> Result<PreparedDatabase, PrivacyError> {
        let encrypt = Self::encryption_wanted(app_handle).await;
        let keys = Self::key_store(app_handle);
        let file = OnDiskDatabase::new(db_path.to_path_buf());
        StartupConversion::new(&file, &keys).prepare(encrypt).await
    }

    /// `privacy.encryptDatabase`.
    async fn encryption_wanted(app_handle: &tauri::AppHandle) -> bool {
        let settings = crate::features::settings::current(app_handle).await;
        settings.privacy.encrypt_database
    }

    /// The Keychain item of the database key, under the bundle identifier.
    fn key_store(app_handle: &tauri::AppHandle) -> KeychainSecretStore {
        KeychainSecretStore::new(app_handle.config().identifier.clone())
    }

    /// Returns true when no database file exists yet (first launch).
    pub async fn is_first_launch(app_handle: &tauri::AppHandle) -> Result<bool, DbError> {
        Ok(!Self::db_path(app_handle).exists())
    }

    fn db_path(app_handle: &tauri::AppHandle) -> PathBuf {
        app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir")
            .join(DB_FILE_NAME)
    }

    /// Deletes the WAL side files next to `db_path`, if present.
    fn remove_wal_side_files(db_path: &Path) {
        for suffix in WAL_SIDE_FILE_SUFFIXES {
            let mut side_file = db_path.as_os_str().to_owned();
            side_file.push(suffix);
            let side_file = PathBuf::from(side_file);
            if !side_file.exists() {
                continue;
            }
            match fs::remove_file(&side_file) {
                Ok(()) => log::info!("Removed orphaned file: {}", side_file.display()),
                Err(e) => log::warn!("Failed to remove {}: {}", side_file.display(), e),
            }
        }
    }

    /// The database, for the composition root.
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Whether the database is encrypted, and what startup did to it.
    pub fn encryption(&self) -> EncryptionReport {
        self.encryption
    }

    /// The writer connection that legacy commands read and write through.
    pub fn pool(&self) -> &SqlitePool {
        self.database.writer()
    }

    /// Checkpoints the WAL into the main file and closes every connection.
    /// Called on application shutdown, so no `-wal`/`-shm` files remain.
    pub async fn cleanup(&self) {
        log::info!("Starting database cleanup...");
        self.database.close().await;
        log::info!("Database connection pool closed");
    }
}
