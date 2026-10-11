//! The one list of pipeline stages.
//!
//! **Adding a stage is one line in [`stages`]**, plus a field in
//! [`StagePorts`] if it needs a port no stage has asked for yet. The runner
//! never names a stage: it runs whatever this list holds, in order.

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::diarization::TrackDiarizer;
use deino_core::export::AutoExporter;
use deino_core::folders::AutoFiler;
use deino_core::meetings::MeetingLifecycleRepo;
use deino_core::people::AttendeeRepo;
use deino_core::segments::{SegmentReader, SegmentWriter, SpeakerRepo};
use deino_core::tags::AutoTagger;
use deino_domain::{Clock, StageId};

use crate::stages::{
    AutoExportStage, AutoFolderStage, AutoTagStage, DiarizeStage, FinalizeAudioStage,
    LegacySummaryStage, Summarizer,
};
use crate::{Job, PipelineStage};

/// The ports the registered stages are built with. The composition root
/// fills it with the app's adapters; each stage takes only what it uses.
#[derive(Clone)]
pub struct StagePorts {
    /// The transcript.
    pub segments: Arc<dyn SegmentReader>,
    /// Meeting state: template, timing, files.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// The legacy summariser (until NOT-02).
    pub summaries: Arc<dyn Summarizer>,
    /// Who was invited (diarization's speaker counts).
    pub attendees: Arc<dyn AttendeeRepo>,
    /// Segment attributions.
    pub segment_writer: Arc<dyn SegmentWriter>,
    /// The meetings' speakers.
    pub speakers: Arc<dyn SpeakerRepo>,
    /// The diarization engine (`deino-diarize` in the app).
    pub diarizer: Arc<dyn TrackDiarizer>,
    /// Writes the meeting into the user's notes folder (EXP-02).
    pub auto_export: Arc<dyn AutoExporter>,
    /// IDs for rows stages create.
    pub clock: Arc<dyn Clock>,
    /// The auto-tagger (ORG-06).
    pub auto_tagger: Arc<dyn AutoTagger>,
    /// The auto-filer (ORG-05).
    pub auto_filer: Arc<dyn AutoFiler>,
    /// The folder holding every live recording's folder (`finalize_audio`).
    pub recordings: PathBuf,
}

#[cfg(any(test, feature = "test-support"))]
impl StagePorts {
    /// Ports over in-memory fakes, for tests that replace the ones they use
    /// (`StagePorts { segments, ..StagePorts::fakes() }`).
    pub fn fakes() -> Self {
        use deino_core::diarization::FakeTrackDiarizer;
        use deino_core::meetings::FakeMeetingRepo;
        use deino_core::people::FakePeopleRepo;
        use deino_core::segments::FakeSegmentRepo;
        use deino_domain::FakeClock;

        let segments = Arc::new(FakeSegmentRepo::default());
        Self {
            segments: segments.clone(),
            meetings: Arc::new(FakeMeetingRepo::default()),
            summaries: Arc::new(crate::stages::RecordedSummaries::default()),
            attendees: Arc::new(FakePeopleRepo::default()),
            segment_writer: segments.clone(),
            speakers: segments,
            diarizer: Arc::new(FakeTrackDiarizer::default()),
            auto_export: Arc::new(crate::stages::ScriptedAutoExporter::default()),
            clock: Arc::new(FakeClock::new(chrono::DateTime::UNIX_EPOCH)),
            auto_tagger: Arc::new(crate::stages::RecordedAutoTags::default()),
            auto_filer: Arc::new(crate::stages::RecordedAutoFiling::default()),
            recordings: PathBuf::from("no-recordings"),
        }
    }
}

/// Every stage of the post-meeting pipeline, sorted by order.
pub fn stages(ports: &StagePorts) -> Vec<Arc<dyn PipelineStage>> {
    sorted(vec![
        // 10 · FND-05b: the recording's mic, system and mixed files.
        Arc::new(FinalizeAudioStage::new(ports)),
        // 30 · CAP-03: speakers from the recording.
        Arc::new(DiarizeStage::new(ports)),
        // 50 · until NOT-02's enhance_notes replaces it.
        Arc::new(LegacySummaryStage::new(ports)),
        // 70 · ORG-06.
        Arc::new(AutoTagStage::new(ports)),
        // 75 · ORG-05.
        Arc::new(AutoFolderStage::new(ports)),
        // 99 · EXP-02: the notes into the user's folder, last.
        Arc::new(AutoExportStage::new(ports)),
    ])
}

/// Stages sorted by order, then by ID so equal orders run predictably.
fn sorted(mut stages: Vec<Arc<dyn PipelineStage>>) -> Vec<Arc<dyn PipelineStage>> {
    stages.sort_by_cached_key(|stage| (stage.order(), stage.id()));
    stages
}

/// The stages the runner runs, in order, looked up by ID.
pub struct Registry {
    stages: Vec<Arc<dyn PipelineStage>>,
}

impl Registry {
    /// A registry of `stages`, sorted by order. A stage whose ID repeats an
    /// earlier one is dropped (and logged): jobs are keyed by stage ID.
    pub fn new(stages: Vec<Arc<dyn PipelineStage>>) -> Self {
        let mut unique: Vec<Arc<dyn PipelineStage>> = Vec::with_capacity(stages.len());
        for stage in sorted(stages) {
            let id = stage.id();
            if unique.iter().any(|kept| kept.id() == id) {
                log::error!(target: "pipeline", "stage {id} is registered twice; keeping the first");
                continue;
            }
            unique.push(stage);
        }
        Self { stages: unique }
    }

    /// The stages, in the order they run.
    pub fn stages(&self) -> &[Arc<dyn PipelineStage>] {
        &self.stages
    }

    /// Every stage's ID, in the order they run.
    pub fn ids(&self) -> Vec<StageId> {
        self.stages.iter().map(|stage| stage.id()).collect()
    }

    /// The stage named `id`, if it is registered.
    pub fn find(&self, id: &StageId) -> Option<&Arc<dyn PipelineStage>> {
        self.stages.iter().find(|stage| stage.id() == *id)
    }

    /// `jobs` of registered stages, in the order the stages run.
    pub fn in_order(&self, jobs: Vec<Job>) -> Vec<Job> {
        let mut ranked: Vec<(usize, Job)> = jobs
            .into_iter()
            .filter_map(|job| self.position(&job.stage).map(|rank| (rank, job)))
            .collect();
        ranked.sort_by_key(|(rank, _)| *rank);
        ranked.into_iter().map(|(_, job)| job).collect()
    }

    fn position(&self, id: &StageId) -> Option<usize> {
        self.stages.iter().position(|stage| stage.id() == *id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeStage, job};

    #[test]
    fn registered_stages_are_sorted_and_uniquely_named() {
        let stages = stages(&StagePorts::fakes());

        let orders: Vec<u16> = stages.iter().map(|stage| stage.order()).collect();
        assert!(orders.is_sorted(), "{orders:?}");
        assert_eq!(Registry::new(stages.clone()).stages().len(), stages.len());
        assert_eq!(
            Registry::new(stages).ids(),
            [
                "finalize_audio",
                "diarize",
                "legacy_summary",
                "auto_tag",
                "auto_folder",
                "auto_export"
            ]
            .map(StageId::new)
        );
    }

    #[test]
    fn the_registry_runs_stages_by_order_whatever_order_they_come_in() {
        let registry = Registry::new(vec![
            FakeStage::succeeding("index", 80).into_dyn(),
            FakeStage::succeeding("finalize_audio", 10).into_dyn(),
            FakeStage::succeeding("diarize", 30).into_dyn(),
        ]);

        let ids: Vec<String> = registry.ids().iter().map(ToString::to_string).collect();
        assert_eq!(ids, ["finalize_audio", "diarize", "index"]);
    }

    #[test]
    fn a_repeated_stage_id_keeps_the_first_by_order() {
        let registry = Registry::new(vec![
            FakeStage::succeeding("notes", 60).into_dyn(),
            FakeStage::succeeding("notes", 50).into_dyn(),
        ]);

        assert_eq!(registry.stages().len(), 1);
        assert_eq!(registry.stages()[0].order(), 50);
    }

    #[test]
    fn jobs_come_back_in_stage_order_without_unregistered_stages() {
        let registry = Registry::new(vec![
            FakeStage::succeeding("a", 10).into_dyn(),
            FakeStage::succeeding("b", 20).into_dyn(),
        ]);
        let jobs = vec![job("b"), job("retired"), job("a")];

        let ordered: Vec<String> = registry
            .in_order(jobs)
            .iter()
            .map(|job| job.stage.to_string())
            .collect();

        assert_eq!(ordered, ["a", "b"]);
    }
}
