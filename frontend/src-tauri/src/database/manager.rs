//! The legacy handle to Deino's database.
//!
//! Opens `deino.sqlite` in the app data dir through [`deino_db::Database`]
//! (WAL, one writer, a read pool, every migration in `crates/deino-db`).
//! Inherited commands reach SQLite through [`DatabaseManager::pool`], which is
//! the writer connection: its commit hooks report every legacy write to the
//! UI as `DbChanged`, so the new TanStack Query caches stay fresh even when
//! legacy code writes. New code receives the `Database` from the composition
//! root instead (`composition.rs`).

use std::fs;
use std::path::{Path, PathBuf};

use deino_db::{Database, DbError, DbLocation, MIGRATOR};
use sqlx::SqlitePool;
use tauri::Manager;

/// File name of Deino's SQLite database inside the app data dir.
pub const DB_FILE_NAME: &str = "deino.sqlite";

/// Suffixes of the SQLite side files that can be orphaned by a crash in WAL mode.
const WAL_SIDE_FILE_SUFFIXES: [&str; 2] = ["-wal", "-shm"];

/// Shared handle to the application's database.
#[derive(Clone, Debug)]
pub struct DatabaseManager {
    database: Database,
}

impl DatabaseManager {
    /// Opens (creating it if needed) the database at `db_path` and runs all migrations.
    pub async fn open(db_path: &Path) -> Result<Self, DbError> {
        let database = Database::open(&DbLocation::File(db_path.to_path_buf()), &MIGRATOR).await?;
        Ok(Self { database })
    }

    /// Opens the database in the app data dir, retrying once after removing
    /// orphaned WAL side files when SQLite reports corruption.
    pub async fn new_from_app_handle(app_handle: &tauri::AppHandle) -> Result<Self, DbError> {
        let db_path = Self::db_path(app_handle);
        log::info!("Database path: {}", db_path.display());

        match Self::open(&db_path).await {
            Ok(db_manager) => Ok(db_manager),
            Err(e) if e.is_corruption() => {
                log::warn!("Database appears corrupted, likely an orphaned WAL file: {:?}", e);
                Self::remove_wal_side_files(&db_path);
                Self::open(&db_path).await
            }
            Err(e) => {
                log::error!("Database connection failed: {:?}", e);
                Err(e)
            }
        }
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
