//! Startup database initialization.

use log::info;
use tauri::AppHandle;

use super::manager::DatabaseManager;
use crate::composition;

/// Opens the database at startup and wires it into the app.
///
/// On first launch there is no database yet, so a fresh one is created here
/// (FND-11): onboarding's steps (profile, calendar, the test recording) need
/// it from the first screen, and the UI no longer creates it.
pub async fn initialize_database_on_startup(app: &AppHandle) -> Result<(), String> {
    let is_first_launch = DatabaseManager::is_first_launch(app)
        .await
        .map_err(|e| format!("Failed to check first launch status: {}", e))?;

    if is_first_launch {
        info!("First launch detected; creating a fresh database");
        return super::commands::initialize_fresh_database(app.clone()).await;
    }

    let db_manager = DatabaseManager::new_from_app_handle(app)
        .await
        .map_err(|e| format!("Failed to initialize database manager: {}", e))?;

    composition::attach_database(app, db_manager).await;
    info!("Database initialized successfully");
    Ok(())
}
