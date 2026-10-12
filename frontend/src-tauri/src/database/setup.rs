//! Startup database initialization.

use std::sync::Arc;

use log::info;
use tauri::{AppHandle, Manager};

use super::manager::DatabaseManager;
use crate::composition;
use crate::features::privacy::library::{LibraryProblem, LibraryStatus, LibraryStatusCell};

/// Opens the database at startup and wires it into the app.
///
/// On first launch there is no database yet, so a fresh one is created here
/// (FND-11): onboarding's steps (profile, calendar, the test recording) need
/// it from the first screen, and the UI no longer creates it.
///
/// When the file must first be encrypted or decrypted (PRV-02), that runs in
/// the background so the window can show a calm progress screen; the UI
/// waits for `LibraryStatus::Ready` before it reads the library.
pub async fn initialize_database_on_startup(app: &AppHandle) -> Result<(), String> {
    let pending = DatabaseManager::pending_conversion(app)
        .await
        .map_err(|e| format!("Failed to inspect the database file: {e}"))?;
    let Some(direction) = pending else {
        return open_library(app).await;
    };
    info!("Converting the database ({direction:?}) before it opens");
    library_status(app).set(LibraryStatus::Converting { direction });
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = open_library(&app).await {
            log::error!("Failed to open the database after converting it: {e}");
        }
    });
    Ok(())
}

/// Opens (or creates) the database and wires it in. A key problem keeps the
/// app running on an explanation screen and touches nothing on disk.
async fn open_library(app: &AppHandle) -> Result<(), String> {
    let is_first_launch = DatabaseManager::is_first_launch(app)
        .await
        .map_err(|e| format!("Failed to check first launch status: {}", e))?;

    if is_first_launch {
        info!("First launch detected; creating a fresh database");
        return super::commands::initialize_fresh_database(app.clone()).await;
    }

    let db_manager = match DatabaseManager::new_from_app_handle(app).await {
        Ok(db_manager) => db_manager,
        Err(error) => {
            let problem = LibraryProblem::of(&error);
            library_status(app).set(LibraryStatus::Unavailable { problem });
            if problem.is_key_problem() {
                log::error!("The database cannot be opened: {error}");
                return Ok(());
            }
            return Err(format!("Failed to initialize database manager: {error}"));
        }
    };

    let encryption = db_manager.encryption();
    composition::attach_database(app, db_manager).await;
    composition::privacy::library_ready(app, encryption);
    info!("Database initialized successfully");
    Ok(())
}

/// The library status the `privacy` plugin reports, managed on first use.
fn library_status(app: &AppHandle) -> Arc<LibraryStatusCell> {
    if let Some(status) = app.try_state::<Arc<LibraryStatusCell>>() {
        return Arc::clone(&status);
    }
    let status = Arc::new(LibraryStatusCell::default());
    app.manage(Arc::clone(&status));
    status
}
