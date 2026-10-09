//! Owns the SQLite connection pool: opening `deino.sqlite` in the app data dir,
//! running the bundled migrations, and closing the pool cleanly on exit.

use sqlx::{migrate::MigrateDatabase, Result, Sqlite, SqlitePool, Transaction};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

/// File name of Deino's SQLite database inside the app data dir.
pub const DB_FILE_NAME: &str = "deino.sqlite";

/// Suffixes of the SQLite side files that can be orphaned by a crash in WAL mode.
const WAL_SIDE_FILE_SUFFIXES: [&str; 2] = ["-wal", "-shm"];

/// Shared handle to the application's SQLite pool.
#[derive(Clone)]
pub struct DatabaseManager {
    pool: SqlitePool,
}

impl DatabaseManager {
    /// Opens (creating it if needed) the database at `db_path` and runs all migrations.
    pub async fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent_dir) = db_path.parent() {
            fs::create_dir_all(parent_dir).map_err(sqlx::Error::Io)?;
        }

        let url = db_path.to_string_lossy();
        if !db_path.exists() {
            log::info!("Creating database at {}", url);
            Sqlite::create_database(&url).await?;
        }

        let pool = SqlitePool::connect(&url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(DatabaseManager { pool })
    }

    /// Opens the database in the app data dir, retrying once after removing
    /// orphaned WAL side files when SQLite reports corruption.
    pub async fn new_from_app_handle(app_handle: &tauri::AppHandle) -> Result<Self> {
        let db_path = Self::db_path(app_handle);
        log::info!("Database path: {}", db_path.display());

        match Self::new(&db_path).await {
            Ok(db_manager) => Ok(db_manager),
            Err(e) if Self::is_corruption_error(&e) => {
                log::warn!("Database appears corrupted, likely an orphaned WAL file: {}", e);
                Self::remove_wal_side_files(&db_path);
                Self::new(&db_path).await
            }
            Err(e) => {
                log::error!("Database connection failed: {}", e);
                Err(e)
            }
        }
    }

    /// Returns true when no database file exists yet (first launch).
    pub async fn is_first_launch(app_handle: &tauri::AppHandle) -> Result<bool> {
        Ok(!Self::db_path(app_handle).exists())
    }

    fn db_path(app_handle: &tauri::AppHandle) -> PathBuf {
        app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir")
            .join(DB_FILE_NAME)
    }

    fn is_corruption_error(error: &sqlx::Error) -> bool {
        let message = error.to_string();
        message.contains("malformed") || message.contains("corrupt")
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

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn with_transaction<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Transaction<'_, Sqlite>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut tx = self.pool.begin().await?;
        let result = f(&mut tx).await;

        match result {
            Ok(val) => {
                tx.commit().await?;
                Ok(val)
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err)
            }
        }
    }

    /// Cleanup database connection and checkpoint WAL
    /// This should be called on application shutdown to ensure:
    /// - All WAL changes are written to the main database file
    /// - The .wal and .shm files are deleted
    /// - Connection pool is gracefully closed
    pub async fn cleanup(&self) -> Result<()> {
        log::info!("Starting database cleanup...");

        // Force checkpoint of WAL to main database file and remove WAL file
        // TRUNCATE mode: checkpoints all pages AND deletes the WAL file
        match sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await
        {
            Ok(_) => log::info!("WAL checkpoint completed successfully"),
            Err(e) => log::warn!("WAL checkpoint failed (non-fatal): {}", e),
        }

        // Close the connection pool gracefully
        self.pool.close().await;
        log::info!("Database connection pool closed");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fresh_database_applies_migrations_without_licensing_table() {
        let dir = tempfile::tempdir().unwrap();
        let manager = DatabaseManager::new(&dir.path().join(DB_FILE_NAME)).await.unwrap();

        let tables: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table'")
                .fetch_all(manager.pool())
                .await
                .unwrap();

        assert!(tables.iter().any(|t| t == "meetings"));
        assert!(!tables.iter().any(|t| t == "licensing"));
    }
}
