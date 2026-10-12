//! Wires the Meetings page (ORG-07): the filtered list, saved views, the
//! trash with its 30-day purge, bulk tagging and filing, and the merge.

use std::sync::Arc;

use deino_core::meetings::{
    DiskMeetingFiles, MeetingListDeps, MeetingListService, MeetingMergeService, MeetingViewService,
    MergeDeps, SqliteMeetingRepo, TrashDeps, TrashPurger, TrashService,
};
use deino_core::notes::SqliteNotesRepo;
use deino_core::settings::SettingsService;
use deino_db::Database;
use deino_domain::{Clock, SystemZone};
use tauri::{AppHandle, Manager};
use tokio_util::sync::CancellationToken;

use super::Organize;
use crate::paths::AppPaths;

/// Stops the trash purge's loop at exit.
pub(crate) struct TrashPurge(CancellationToken);

impl TrashPurge {
    /// Ends the purge loop; a purge in progress finishes first.
    pub(crate) fn stop(&self) {
        self.0.cancel();
    }
}

/// Builds and manages the Meetings page's services, and starts the purge.
/// Without the app folders (a broken install) the trash cannot delete
/// recordings, so it and the purge are left out; the list still works.
pub(crate) fn attach_meetings(
    app: &AppHandle,
    database: &Database,
    clock: &Arc<dyn Clock>,
    (organize, settings): (&Organize, &Arc<SettingsService>),
) {
    let repo = Arc::new(SqliteMeetingRepo::new(database.clone()));
    app.manage(Arc::new(MeetingListService::new(MeetingListDeps {
        repo: repo.clone(),
        zone: Arc::new(SystemZone),
        addresses: Arc::clone(settings) as _,
    })));
    app.manage(Arc::new(MeetingViewService::new(
        repo.clone(),
        Arc::clone(clock),
    )));
    app.manage(Arc::new(organize.bulk()));
    app.manage(Arc::new(MeetingMergeService::new(MergeDeps {
        repo: repo.clone(),
        notes: Arc::new(SqliteNotesRepo::new(database.clone())),
        clock: Arc::clone(clock),
    })));
    attach_trash(app, repo, clock);
}

/// The trash over the app's recordings and note folders, and its purge.
fn attach_trash(app: &AppHandle, repo: Arc<SqliteMeetingRepo>, clock: &Arc<dyn Clock>) {
    let paths = match AppPaths::resolve(app) {
        Ok(paths) => paths,
        Err(error) => {
            return log::error!(target: "meetings", "no app folders, the trash is unavailable: {error}");
        }
    };
    let trash = Arc::new(TrashService::new(TrashDeps {
        bin: repo,
        files: Arc::new(DiskMeetingFiles::new(paths.recordings(), paths.meetings())),
        clock: Arc::clone(clock),
    }));
    let (purger, cancel) = TrashPurger::new(Arc::clone(&trash));
    // Owned for the app's lifetime; `TrashPurge::stop` ends it at exit.
    tauri::async_runtime::spawn(purger.run());
    app.manage(trash);
    app.manage(TrashPurge(cancel));
}
