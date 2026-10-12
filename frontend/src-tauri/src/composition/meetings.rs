//! Wires the Meetings page (ORG-07): the filtered list, saved views, the
//! trash with its 30-day purge, bulk tagging and filing, and the merge.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::meetings::{
    DiskMeetingFiles, MeetingListDeps, MeetingListService, MeetingMergeService, MeetingViewService,
    MeetingWork, MergeDeps, SqliteMeetingRepo, TrashDeps, TrashPurger, TrashService,
};
use deino_core::notes::SqliteNotesRepo;
use deino_core::settings::SettingsService;
use deino_db::Database;
use deino_domain::{Clock, MeetingId, StageId, SystemZone};
use deino_pipeline::PipelineService;
use deino_pipeline::stages::INDEX_STAGE;
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
    let work: Arc<dyn MeetingWork> = Arc::new(PipelineWork(app.clone()));
    app.manage(Arc::new(MeetingMergeService::new(MergeDeps {
        repo: repo.clone(),
        notes: Arc::new(SqliteNotesRepo::new(database.clone())),
        clock: Arc::clone(clock),
        work: Arc::clone(&work),
    })));
    attach_trash(app, repo, (clock, work));
}

/// [`MeetingWork`] over the post-meeting pipeline. It looks the pipeline up
/// when called, because the pipeline is attached after these services.
struct PipelineWork(AppHandle);

impl PipelineWork {
    fn pipeline(&self) -> Option<Arc<PipelineService>> {
        let pipeline = self.0.try_state::<Arc<PipelineService>>()?;
        Some(Arc::clone(&pipeline))
    }
}

#[async_trait]
impl MeetingWork for PipelineWork {
    fn cancel(&self, meeting: &MeetingId) {
        if let Some(pipeline) = self.pipeline() {
            pipeline.cancel(meeting);
        }
    }

    async fn reindex(&self, meeting: &MeetingId) {
        let Some(pipeline) = self.pipeline() else {
            return;
        };
        let index = StageId::new(INDEX_STAGE);
        if let Err(error) = pipeline.rerun(meeting, Some(&index)).await {
            log::warn!(target: "meetings", "could not re-index merged meeting {meeting}: {error}");
        }
    }
}

/// The trash over the app's recordings and note folders, and its purge.
fn attach_trash(
    app: &AppHandle,
    repo: Arc<SqliteMeetingRepo>,
    (clock, work): (&Arc<dyn Clock>, Arc<dyn MeetingWork>),
) {
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
        work,
    }));
    let (purger, cancel) = TrashPurger::new(Arc::clone(&trash));
    // Owned for the app's lifetime; `TrashPurge::stop` ends it at exit.
    tauri::async_runtime::spawn(purger.run());
    app.manage(trash);
    app.manage(TrashPurge(cancel));
}
