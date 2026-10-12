//! Wires the post-meeting pipeline (FND-08): the registered stages over the
//! app's repositories and adapters, the meetings lifecycle that starts it,
//! and its startup.

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::action_items::extraction::ActionItemExtractor;
use deino_core::automations::AutomationRunner;
use deino_core::diarization::{SpeakerIdentifier, TrackDiarizer};
use deino_core::entities::EntityUpdater;
use deino_core::export::AutoExporter;
use deino_core::final_transcription::TrackTranscriber;
use deino_core::folders::{AutoFileService, FolderPorts, SqliteFolderRepo};
use deino_core::followups::FollowUpDrafter;
use deino_core::insights::InsightGenerator;
use deino_core::meetings::{MeetingLifecycleService, SqliteMeetingRepo};
use deino_core::people::SqlitePeopleRepo;
use deino_core::search::SearchIndexing;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::tags::AutoTagger;
use deino_core::voiceprints::SqliteVoiceprintRepo;
use deino_db::Database;
use deino_domain::Clock;
use deino_pipeline::{
    AutomaticNotes, ClockSleeper, MeetingTitler, NotesEnhancer, PipelineDeps, PipelineService,
    PipelineTrigger, ProgressSink, RepassSetting, ScreenContextReader, SqliteJobRepo, StagePorts, VocabFixer,
    registry,
};
use tauri::{AppHandle, Manager, Runtime};

use super::{diarize, transcript};
use crate::features;
use crate::paths::AppPaths;

/// What the pipeline gets from the app: where progress goes, the
/// diarization engine behind `diarize`, the services behind the speaker,
/// AI and export stages, the transcript stages' services, and the live
/// recordings' folder that `finalize_audio` reads.
pub(crate) struct PipelineAdapters {
    pub(crate) progress: Arc<dyn ProgressSink>,
    pub(crate) diarizer: Arc<dyn TrackDiarizer>,
    pub(crate) ai: StageServices,
    pub(crate) transcript: TranscriptStages,
    /// Every live recording's folder lives here (`AppPaths::recordings`).
    pub(crate) recordings: PathBuf,
    /// The shared screen's text (`screen_context`, CAP-09).
    pub(crate) screen_context: Arc<dyn ScreenContextReader>,
}

/// The services behind `final_transcription` (CAP-08) and `vocab_fixups`
/// (TRN-05).
pub(crate) struct TranscriptStages {
    /// Transcribes finished tracks.
    pub(crate) transcriber: Arc<dyn TrackTranscriber>,
    /// Settings → Transcription → "High-accuracy re-pass".
    pub(crate) repass: Arc<dyn RepassSetting>,
    /// The vocabulary's `FixupService`.
    pub(crate) vocab_fixer: Arc<dyn VocabFixer>,
}

/// The AI and export stages' services, built by the composition root.
pub(crate) struct StageServices {
    /// `enhance_notes` (NOT-02, `NotesAi::enhancer`).
    pub(crate) enhancer: Arc<dyn NotesEnhancer>,
    /// `auto_title` (NOT-02, `NotesAi::titler`).
    pub(crate) titler: Arc<dyn MeetingTitler>,
    /// "Generate notes automatically" (the settings).
    pub(crate) automatic: Arc<dyn AutomaticNotes>,
    /// The auto-tagger, the exporter, the action item extraction and the speaker identifier.
    pub(crate) adapters: StageAdapters,
}

/// The stage services built before the notes: the auto-tagger (`auto_tag`,
/// ORG-06), the auto-exporter, the action item extraction (AI-05) and the
/// speaker identifier (`identify_speakers`, CAP-04).
pub(crate) struct StageAdapters {
    pub(crate) auto_tagger: Arc<dyn AutoTagger>,
    /// The vault exporter (`ExportServices::auto_exporter`, EXP-02).
    pub(crate) auto_export: Arc<dyn AutoExporter>,
    pub(crate) action_items: Arc<dyn ActionItemExtractor>,
    /// The insights generator (`insights` stage, AI-06).
    pub(crate) insights: Arc<dyn InsightGenerator>,
    /// The automations engine (`automations` stage, AUT-01).
    pub(crate) automations: Arc<dyn AutomationRunner>,
    /// Suggested follow-up emails (`follow_up_draft` stage, NOT-07).
    pub(crate) follow_ups: Arc<dyn FollowUpDrafter>,
    /// Over the settings' thresholds (`Speakers::identifier`, CAP-04).
    pub(crate) identifier: Arc<dyn SpeakerIdentifier>,
    pub(crate) search: Arc<dyn SearchIndexing>,
    /// People and companies after a meeting (`composition/entities.rs`, ORG-04).
    pub(crate) entities: Arc<dyn EntityUpdater>,
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
        enhancer: adapters.ai.enhancer,
        titler: adapters.ai.titler,
        automatic: adapters.ai.automatic,
        attendees: Arc::new(SqlitePeopleRepo::new(database.clone())),
        segment_writer: segments.clone(),
        speakers: segments,
        diarizer: adapters.diarizer,
        voiceprints: Arc::new(SqliteVoiceprintRepo::new(database.clone())),
        identifier: adapters.ai.adapters.identifier,
        clock: Arc::clone(clock),
        auto_tagger: adapters.ai.adapters.auto_tagger,
        auto_export: adapters.ai.adapters.auto_export,
        action_items: adapters.ai.adapters.action_items,
        insights: adapters.ai.adapters.insights,
        automations: adapters.ai.adapters.automations,
        follow_ups: adapters.ai.adapters.follow_ups,
        auto_filer: Arc::new(AutoFileService::new(&folders)),
        recordings: adapters.recordings,
        search: adapters.ai.adapters.search,
        entities: adapters.ai.adapters.entities,
        transcriber: adapters.transcript.transcriber,
        repass: adapters.transcript.repass,
        vocab_fixer: adapters.transcript.vocab_fixer,
        screen_context: adapters.screen_context,
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
/// Needs the diarization models, the settings and the vocabulary.
pub(super) fn attach_pipeline(
    app: &AppHandle,
    database: &Database,
    clock: &Arc<dyn Clock>,
    ai: StageServices,
) {
    let recordings = match AppPaths::resolve(app) {
        Ok(paths) => paths.recordings(),
        Err(error) => {
            return log::error!(target: "pipeline", "app folders unavailable: {error}; the pipeline is off");
        }
    };
    let diarizer = match diarize::track_diarizer(app) {
        Ok(diarizer) => diarizer,
        Err(error) => {
            return log::error!(target: "pipeline", "diarization could not start: {error}; the pipeline is off");
        }
    };
    let Some(transcript) = transcript::stages(app) else {
        return log::error!(target: "pipeline", "settings or the vocabulary are not ready; the pipeline is off");
    };
    let adapters = PipelineAdapters {
        screen_context: super::screen::context_reader(app, database, &recordings),
        diarizer,
        progress: Arc::new(features::pipeline::TauriPipelineProgress::new(app.clone())),
        ai,
        transcript,
        recordings,
    };
    Pipeline::build(database, clock, adapters).start_and_manage(app);
}
