//! The composition root: the one place that turns adapters into services.
//!
//! Services in `deino-core` depend on ports (traits). This module picks the
//! implementations (SQLite repositories, the system clock, Tauri event
//! adapters) and registers each service as Tauri state, so a command receives
//! `State<'_, Arc<Service>>` and never builds anything itself (§3A dependency
//! inversion). Tests build the same services over fakes instead.
//!
//! Startup wires two groups, in order:
//!
//! 1. [`attach_settings`]: settings and Keychain secrets, which need no
//!    database and which everything else reads;
//! 2. [`attach_database`]: the database-backed [`Services`], once
//!    `deino.sqlite` is open (its legacy settings are imported on the way
//!    by [`legacy_settings_import`]).
//!
//! A new feature adds its service to [`Services`]: one field, one line in
//! [`Services::build`] and one in [`Services::manage`].

use std::sync::Arc;

use deino_core::meetings::{MeetingService, SqliteMeetingRepo};
use deino_core::settings::{
    JsonFileSettingsStore, LegacyFileImport, LegacySqliteImport, LlmAccess, SettingsError,
    SettingsService,
};
use deino_db::Database;
use deino_domain::{Clock, SystemClock};
use deino_secrets::{KeychainSecretStore, SecretStore};
use tauri::{AppHandle, Manager, Runtime};

use crate::database::manager::DatabaseManager;
use crate::features;
use crate::features::settings::TauriSettingsEvents;
use crate::paths::AppPaths;
use crate::state::AppState;

/// Loads the settings, moves the legacy JSON stores into them, opens the
/// Keychain, and manages `Arc<SettingsService>` and `Arc<dyn SecretStore>`.
///
/// Runs first at startup. Keychain items carry the bundle identifier as
/// their service (`com.xenshreyas.deino`), so a build with another
/// identifier never sees this app's keys.
pub(crate) async fn attach_settings(
    app: &AppHandle,
    paths: &AppPaths,
) -> Result<(), SettingsError> {
    let store = Arc::new(JsonFileSettingsStore::new(paths.settings_file()));
    let events = Arc::new(TauriSettingsEvents::new(app.clone()));
    let settings = Arc::new(SettingsService::load(store, events).await?);
    // A store that cannot be imported keeps its defaults; it never blocks startup.
    if let Err(error) = LegacyFileImport::new(paths.data()).run(&settings).await {
        log::warn!(target: "settings", "could not import the legacy settings files: {error}");
    }
    let secrets: Arc<dyn SecretStore> =
        Arc::new(KeychainSecretStore::new(app.config().identifier.clone()));
    app.manage(settings);
    app.manage(secrets);
    Ok(())
}

/// The data migration that moves the legacy SQLite settings and keys into
/// the settings file and the Keychain before their tables are dropped.
/// `None` until [`attach_settings`] has run.
pub(crate) fn legacy_settings_import<R: Runtime>(app: &AppHandle<R>) -> Option<LegacySqliteImport> {
    let settings = app.try_state::<Arc<SettingsService>>()?;
    let secrets = app.try_state::<Arc<dyn SecretStore>>()?;
    Some(LegacySqliteImport::new(
        Arc::clone(&settings),
        Arc::clone(&secrets),
    ))
}

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
    attach_llm(app, &database, clock);
    #[cfg(debug_assertions)]
    attach_fixture_seeder(app, &database);
    features::db::forward_changes(app, database.changes());
}

/// Wires the LLM gateway (`crate::llm`): manages `Arc<Gateway>`,
/// `Arc<LlmAdmin>` and `Arc<SummaryGenerator>`, and starts the usage writer
/// that saves `llm_calls`. Needs [`attach_settings`] to have run.
fn attach_llm(app: &AppHandle, database: &Database, clock: Arc<dyn Clock>) {
    let (Some(settings), Some(secrets)) = (
        app.try_state::<Arc<SettingsService>>(),
        app.try_state::<Arc<dyn SecretStore>>(),
    ) else {
        log::error!(target: "llm", "settings were not wired before the database; AI is unavailable");
        return;
    };
    let access = Arc::new(LlmAccess::new(Arc::clone(&settings), Arc::clone(&secrets)));
    let app_data_dir = app.path().app_data_dir().unwrap_or_default();
    let services = crate::llm::build(crate::llm::LlmPorts {
        access,
        database: database.clone(),
        app_data_dir,
        clock,
    });
    // Owned for the app's lifetime: it ends when the gateway, and with it the
    // last usage channel, is dropped at exit.
    tauri::async_runtime::spawn(services.usage_writer.run());
    app.manage(services.gateway);
    app.manage(services.admin);
    app.manage(services.summaries);
    let pull_events: Arc<dyn features::llm::PullEvents> =
        Arc::new(features::llm::TauriPullEvents::new(app.clone()));
    app.manage(pull_events);
}

/// Debug builds only: gives the `dev` plugin a seeder that writes the fixture
/// meetings into the app's database (Settings → Developer).
#[cfg(debug_assertions)]
fn attach_fixture_seeder(app: &AppHandle, database: &Database) {
    let seeder = deino_testkit::FixtureSeeder::new(database.clone());
    app.manage(Arc::new(seeder));
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
