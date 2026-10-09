//! Wiring for calendar sync (ORG-01): macOS Calendar through `EventKit`,
//! the SQLite calendar, people and company repositories, the sync
//! scheduler, and the `CalendarChanged` event adapter.

use std::sync::Arc;

use deino_core::calendar::{
    AttendeeDirectory, CalendarDeps, CalendarService, CalendarStore, SqliteCalendarRepo,
    SyncScheduler,
};
use deino_core::companies::SqliteCompanyRepo;
use deino_core::people::SqlitePeopleRepo;
use deino_core::settings::SettingsService;
use deino_db::Database;
use deino_domain::Clock;
use deino_eventkit::EventKitSource;
use tauri::{AppHandle, Manager};
use tokio_util::sync::CancellationToken;

use crate::features::calendar::TauriCalendarEvents;

/// Stops the calendar sync scheduler at exit.
pub(crate) struct CalendarSchedule(CancellationToken);

impl CalendarSchedule {
    /// Ends the scheduler's loop; a sync in progress finishes first.
    pub(crate) fn stop(&self) {
        self.0.cancel();
    }
}

/// Builds the calendar service over `database`, starts its scheduler (which
/// syncs only while the user has sync on) and manages `Arc<CalendarService>`
/// and the scheduler's [`CalendarSchedule`]. Needs the settings.
pub(crate) fn attach_calendar(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    let Some(settings) = app.try_state::<Arc<SettingsService>>() else {
        return log::error!(target: "calendar", "settings are not loaded; calendar sync is off");
    };
    let source = Arc::new(EventKitSource::new());
    let repo = Arc::new(SqliteCalendarRepo::new(database.clone()));
    let directory = AttendeeDirectory::new(
        Arc::new(SqlitePeopleRepo::new(database.clone())),
        Arc::new(SqliteCompanyRepo::new(database.clone())),
        Arc::clone(clock),
    );
    let service = Arc::new(CalendarService::new(CalendarDeps {
        source: source.clone(),
        store: CalendarStore::over(&repo),
        directory,
        settings: Arc::clone(&settings),
        events: Arc::new(TauriCalendarEvents::new(app.clone())),
        clock: Arc::clone(clock),
    }));
    let (scheduler, triggers, cancel) = SyncScheduler::new(Arc::clone(&service));
    source.watch(triggers);
    // Owned for the app's lifetime; `CalendarSchedule::stop` ends it at exit.
    tauri::async_runtime::spawn(scheduler.run());
    app.manage(service);
    app.manage(CalendarSchedule(cancel));
}
