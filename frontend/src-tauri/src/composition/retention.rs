//! Wires retention (PRV-01): `RetentionService` over SQLite, the settings,
//! the trash and the app's recordings folder, managed for the `retention`
//! plugin. Its sweep runs as housekeeping on the trash purge's loop
//! (`meetings::attach_trash`), so it is built there rather than from an
//! attach hook.

use std::sync::Arc;

use deino_core::meetings::{DiskMeetingFiles, MeetingWork, TrashService};
use deino_core::retention::{RetentionDeps, RetentionService, SqliteRetentionStore};
use deino_core::settings::SettingsService;
use deino_db::Database;
use deino_domain::Clock;
use tauri::{AppHandle, Manager};

/// What retention is wired over, besides the trash and its files.
pub(super) struct RetentionWiring<'a> {
    /// The app's database.
    pub(super) database: &'a Database,
    /// The settings, whose `privacy.retention` it follows.
    pub(super) settings: &'a Arc<SettingsService>,
    /// The time ages are counted from.
    pub(super) clock: &'a Arc<dyn Clock>,
    /// The pipeline, to re-index a meeting whose transcript went.
    pub(super) work: Arc<dyn MeetingWork>,
}

/// Builds and manages `Arc<RetentionService>`; the caller runs its sweep.
pub(super) fn attach_retention(
    app: &AppHandle,
    wiring: RetentionWiring<'_>,
    (trash, files): (&Arc<TrashService>, Arc<DiskMeetingFiles>),
) -> Arc<RetentionService> {
    let service = Arc::new(RetentionService::new(RetentionDeps {
        store: Arc::new(SqliteRetentionStore::new(wiring.database.clone())),
        policy: Arc::clone(wiring.settings) as _,
        files,
        trash: Arc::clone(trash) as _,
        work: wiring.work,
        clock: Arc::clone(wiring.clock),
    }));
    app.manage(Arc::clone(&service));
    service
}
