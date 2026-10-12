//! Wiring for imports and watched folders (CAP-11).
//!
//! `ImportService` over `deino-recording`'s `NativeIngest` (symphonia, then
//! `AVAssetReader`), the meetings repository and the lifecycle service whose
//! `processing` starts the pipeline; `WatchService` over the ledger, the
//! local file system and that import service; and the folder scheduler,
//! started here and stopped at exit ([`WatchSchedule`]).

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::import::{ImportDeps, ImportService, MediaPicker};
use deino_core::meetings::{MeetingLifecycleService, SqliteMeetingRepo};
use deino_core::watch::{
    FolderPicker, LocalWatchFs, SqliteWatchRepo, WatchDeps, WatchNudge, WatchScheduler,
    WatchService,
};
use deino_db::Database;
use deino_domain::Clock;
use deino_recording::NativeIngest;
use tauri::{AppHandle, Manager, Runtime};
use tokio_util::sync::CancellationToken;

use super::attach::Phase;
use crate::features::import::TauriImportDialogs;
use crate::paths::AppPaths;

/// Stops the watched-folder scheduler at exit.
pub(crate) struct WatchSchedule(CancellationToken);

impl WatchSchedule {
    /// Ends the scheduler's loop; an import in progress finishes first.
    pub(crate) fn stop(&self) {
        self.0.cancel();
    }
}

/// The import feature's services and dialogs.
pub(crate) struct Imports {
    pub(crate) service: Arc<ImportService>,
    pub(crate) watch: Arc<WatchService>,
    pub(crate) media: Arc<dyn MediaPicker>,
    pub(crate) folders: Arc<dyn FolderPicker>,
}

impl Imports {
    /// The services over `database`, putting meeting folders in
    /// `recordings` and starting the pipeline through `lifecycle`.
    pub(crate) fn build(
        database: &Database,
        clock: &Arc<dyn Clock>,
        (recordings, lifecycle): (PathBuf, Arc<MeetingLifecycleService>),
        dialogs: (Arc<dyn MediaPicker>, Arc<dyn FolderPicker>),
    ) -> Self {
        let service = Arc::new(ImportService::new(ImportDeps {
            ingest: Arc::new(NativeIngest::new()),
            meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
            lifecycle,
            recordings,
            clock: Arc::clone(clock),
        }));
        let watch = Arc::new(WatchService::new(WatchDeps {
            repo: Arc::new(SqliteWatchRepo::new(database.clone())),
            fs: Arc::new(LocalWatchFs),
            importer: service.clone(),
            clock: Arc::clone(clock),
        }));
        Self {
            service,
            watch,
            media: dialogs.0,
            folders: dialogs.1,
        }
    }

    /// Manages the services, the dialogs and `nudge`.
    pub(crate) fn manage<R: Runtime>(self, app: &impl Manager<R>, nudge: WatchNudge) {
        app.manage(self.service);
        app.manage(self.watch);
        app.manage(self.media);
        app.manage(self.folders);
        app.manage(nudge);
    }
}

super::attach::attach!(
    "import",
    40,
    Phase::Background(|cx| {
        attach_import(cx.app, cx.database, cx.clock);
    })
);

/// Builds and manages the import services and starts the folder scheduler.
/// Needs the pipeline's lifecycle service to be managed first.
pub(crate) fn attach_import(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    let recordings = match AppPaths::resolve(app) {
        Ok(paths) => paths.recordings(),
        Err(error) => {
            return log::error!(target: "import", "app folders unavailable; imports are off: {error}");
        }
    };
    let Some(lifecycle) = app.try_state::<Arc<MeetingLifecycleService>>() else {
        return log::error!(target: "import", "the pipeline is not ready; imports are off");
    };
    let dialogs = Arc::new(TauriImportDialogs::new(app.clone()));
    let imports = Imports::build(
        database,
        clock,
        (recordings, Arc::clone(&*lifecycle)),
        (dialogs.clone(), dialogs),
    );
    let (scheduler, nudge, cancel) = WatchScheduler::new(imports.watch.clone());
    // Owned for the app's lifetime; `WatchSchedule::stop` ends it at exit.
    tauri::async_runtime::spawn(scheduler.run());
    imports.manage(app, nudge);
    app.manage(WatchSchedule(cancel));
}
