//! Wires the post-meeting pipeline (FND-08): the registered stages over the
//! app's repositories and adapters, the meetings lifecycle that starts it,
//! and its startup.

use std::sync::Arc;

use deino_core::diarization::TrackDiarizer;
use deino_core::folders::{AutoFileService, FolderPorts, SqliteFolderRepo};
use deino_core::meetings::{MeetingLifecycleService, SqliteMeetingRepo};
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::SettingsService;
use deino_core::tags::AutoTagger;
use deino_db::Database;
use deino_domain::Clock;
use deino_pipeline::{
    ClockSleeper, PipelineDeps, PipelineService, PipelineTrigger, ProgressSink, SqliteJobRepo,
    StagePorts, Summarizer, registry,
};
use tauri::{AppHandle, Manager, Runtime};

use super::diarize;
use crate::features;

/// What the pipeline gets from the app: where progress goes, the legacy
/// summary service behind `legacy_summary`, the diarization engine behind
/// `diarize`, and the auto-tagger behind `auto_tag`.
pub(crate) struct PipelineAdapters {
    pub(crate) progress: Arc<dyn ProgressSink>,
    pub(crate) summaries: Arc<dyn Summarizer>,
    pub(crate) diarizer: Arc<dyn TrackDiarizer>,
    /// Over the LLM gateway (`Organize::auto_tagger`).
    pub(crate) auto_tagger: Arc<dyn AutoTagger>,
}

/// The ports the registered stages read: SQLite repositories over
/// `database`, the auto-filer over the folder rules, and the app adapters.
fn stage_ports(
    database: &Database,
    clock: &Arc<dyn Clock>,
    meetings: &Arc<SqliteMeetingRepo>,
    adapters: PipelineAdapters,
) -> StagePorts {
    let segments = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let folders = FolderPorts::over(
        Arc::new(SqliteFolderRepo::new(database.clone())),
        Arc::clone(clock),
    );
    StagePorts {
        segments: segments.clone(),
        meetings: meetings.clone(),
        summaries: adapters.summaries,
        attendees: Arc::new(SqlitePeopleRepo::new(database.clone())),
        segment_writer: segments.clone(),
        speakers: segments,
        diarizer: adapters.diarizer,
        clock: Arc::clone(clock),
        auto_tagger: adapters.auto_tagger,
        auto_filer: Arc::new(AutoFileService::new(&folders)),
    }
}

/// The meetings lifecycle and the post-meeting pipeline it starts (FND-08).
pub(crate) struct Pipeline {
    pub(super) lifecycle: Arc<MeetingLifecycleService>,
    pub(super) pipeline: Arc<PipelineService>,
}

impl Pipeline {
    /// Builds the pipeline over `database` with the registered stages, and
    /// subscribes it to the lifecycle so `processing` meetings enqueue.
    pub(crate) fn build(
        database: &Database,
        clock: &Arc<dyn Clock>,
        adapters: PipelineAdapters,
    ) -> Self {
        let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
        let lifecycle = Arc::new(MeetingLifecycleService::new(
            meetings.clone(),
            Arc::clone(clock),
        ));
        let progress = Arc::clone(&adapters.progress);
        let ports = stage_ports(database, clock, &meetings, adapters);
        let pipeline = Arc::new(PipelineService::new(PipelineDeps {
            jobs: Arc::new(SqliteJobRepo::new(database.clone())),
            stages: registry::stages(&ports),
            lifecycle: Arc::clone(&lifecycle),
            meetings,
            clock: Arc::clone(clock),
            sleeper: Arc::new(ClockSleeper::new(Arc::clone(clock))),
            progress,
        }));
        lifecycle.subscribe(Arc::new(PipelineTrigger::new(&pipeline)));
        Self {
            lifecycle,
            pipeline,
        }
    }

    /// Starts the runner (reconciling the queue in the background) and
    /// registers the lifecycle service and the pipeline as Tauri state.
    pub(crate) fn start_and_manage<R: Runtime>(self, app: &impl Manager<R>) {
        self.pipeline.start();
        app.manage(self.lifecycle);
        app.manage(self.pipeline);
    }
}

/// Builds the pipeline with the app's adapters, starts it and manages it.
/// Needs the settings (`attach_settings` runs first), which name the
/// summary model the `legacy_summary` stage uses, and the diarization models.
pub(super) fn attach_pipeline(
    app: &AppHandle,
    database: &Database,
    clock: &Arc<dyn Clock>,
    auto_tagger: Arc<dyn AutoTagger>,
) {
    let Some(settings) = app.try_state::<Arc<SettingsService>>() else {
        return log::error!(target: "pipeline", "settings are not loaded; the pipeline is off");
    };
    let diarizer = match diarize::track_diarizer(app) {
        Ok(diarizer) => diarizer,
        Err(error) => {
            return log::error!(target: "pipeline", "diarization could not start: {error}; the pipeline is off");
        }
    };
    let adapters = PipelineAdapters {
        diarizer,
        progress: Arc::new(features::pipeline::TauriPipelineProgress::new(app.clone())),
        summaries: Arc::new(features::pipeline::LegacySummarizer::new(
            app.clone(),
            database.writer().clone(),
            Arc::clone(&settings),
            Arc::clone(clock),
        )),
        auto_tagger,
    };
    Pipeline::build(database, clock, adapters).start_and_manage(app);
}
