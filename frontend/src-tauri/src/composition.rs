//! The composition root: the one place that turns adapters into services.
//!
//! Services in `deino-core` depend on ports (traits). This module picks the
//! implementations (SQLite repositories, the system clock, Tauri event
//! adapters) and registers each service as Tauri state, so a command receives
//! `State<'_, Arc<Service>>` and never builds anything itself (§3A dependency
//! inversion). Tests build the same services over fakes instead.
//!
//! A new feature adds its service to [`Services`]: one field, one line in
//! [`Services::build`] and one in [`Services::manage`].

use std::sync::Arc;

use deino_core::meetings::{MeetingService, SqliteMeetingRepo};
use deino_db::Database;
use deino_domain::{Clock, SystemClock};
use tauri::{AppHandle, Manager, Runtime};

use crate::database::manager::DatabaseManager;
use crate::features;
use crate::state::AppState;

/// Every feature service backed by the app's database.
pub(crate) struct Services {
    meetings: Arc<MeetingService>,
}

impl Services {
    /// Builds every service over `database`, reading time from `clock`.
    pub(crate) fn build(database: &Database, clock: &Arc<dyn Clock>) -> Self {
        let meeting_repo = Arc::new(SqliteMeetingRepo::new(database.clone()));
        Self {
            meetings: Arc::new(MeetingService::new(meeting_repo, Arc::clone(clock))),
        }
    }

    /// Registers each service as Tauri state, where commands find it.
    pub(crate) fn manage<R: Runtime>(self, app: &impl Manager<R>) {
        app.manage(self.meetings);
    }
}

/// Wires a newly opened database into the app: the legacy `AppState`, every
/// feature service and the `DbChanged` bridge to the UI.
///
/// Runs once, at startup or from the first-launch `initialize_fresh_database`
/// command. Feature commands called before that fail with Tauri's "state not
/// managed" error, as the legacy commands always have.
pub(crate) fn attach_database(app: &AppHandle, manager: DatabaseManager) {
    let database = manager.database().clone();
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    app.manage(AppState {
        db_manager: manager,
    });
    Services::build(&database, &clock).manage(app);
    features::db::forward_changes(app, database.changes());
}

/// Wires the debug-only reference feature over its private in-memory database.
#[cfg(debug_assertions)]
pub(crate) async fn attach_example(app: &AppHandle) -> Result<(), deino_db::DbError> {
    use deino_core::_example::{ExampleService, MIGRATOR, SqliteExampleNoteRepo};
    use deino_db::DbLocation;

    let database = Database::open(&DbLocation::Memory, &MIGRATOR).await?;
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

#[cfg(test)]
mod tests {
    use deino_testkit::{FakeClock, at, seed, test_db};

    use super::*;

    #[tokio::test]
    async fn services_read_through_the_database_they_are_built_over() {
        let database = test_db().await;
        seed::meeting(
            &database,
            "meeting-1",
            "Standup",
            at("2026-10-08T09:00:00Z"),
        )
        .await;
        let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-08T10:00:00Z")));

        let services = Services::build(&database, &clock);

        let meetings = services.meetings.list().await.unwrap();
        assert_eq!(meetings.len(), 1);
    }
}
