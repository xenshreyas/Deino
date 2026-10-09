//! Startup database initialization.

use log::info;
use tauri::AppHandle;

use super::manager::DatabaseManager;
use crate::composition;

/// Opens the existing database at startup and wires it into the app.
///
/// On first launch there is no database yet; the onboarding flow creates it
/// through the `initialize_fresh_database` command, so this does nothing.
pub async fn initialize_database_on_startup(app: &AppHandle) -> Result<(), String> {
    let is_first_launch = DatabaseManager::is_first_launch(app)
        .await
        .map_err(|e| format!("Failed to check first launch status: {}", e))?;

    if is_first_launch {
        info!("First launch detected; onboarding will initialize the database");
        return Ok(());
    }

    let db_manager = DatabaseManager::new_from_app_handle(app)
        .await
        .map_err(|e| format!("Failed to initialize database manager: {}", e))?;

    composition::attach_database(app, db_manager);
    info!("Database initialized successfully");
    Ok(())
}
