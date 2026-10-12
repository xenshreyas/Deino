//! Wiring for the debug-only reference feature (`_example`): its service over
//! a private in-memory database, registered as a startup hook from here, so
//! the feature needs no line in `composition.rs` or `setup.rs`.

#![cfg(debug_assertions)]

use std::sync::Arc;

use deino_core::_example::{ExampleService, MIGRATOR, SqliteExampleNoteRepo};
use deino_db::{Database, DbLocation};
use deino_domain::SystemClock;
use tauri::{AppHandle, Manager};

use super::attach::Phase;
use crate::features;

super::attach::attach!("example", 10, Phase::Startup(attach_example));

/// Opens the example's in-memory database and manages `Arc<ExampleService>`
/// over it, with its `DbChanged` bridge to the UI.
fn attach_example(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let database = tauri::async_runtime::block_on(Database::open(&DbLocation::Memory, &MIGRATOR))?;
    let repo = Arc::new(SqliteExampleNoteRepo::new(database.clone()));
    let events = Arc::new(features::_example::TauriExampleEvents::new(app.clone()));
    app.manage(Arc::new(ExampleService::new(
        repo,
        events,
        Arc::new(SystemClock),
    )));
    features::db::forward_changes(app, database.changes());
    Ok(())
}
