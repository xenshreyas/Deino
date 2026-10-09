//! A pipeline over a real database, a fake clock and fake stages, wired the
//! way the app wires it: the trigger subscribed to the lifecycle service.

use std::sync::Arc;

use deino_core::meetings::{
    MeetingLifecycleRepo, MeetingLifecycleService, MeetingTitle, NewMeeting, SqliteMeetingRepo,
};
use deino_db::{Database, test_db};
use deino_domain::{FakeClock, JobStatus, MeetingId, MeetingSource, MeetingStatus, StageId};
use deino_testkit::at;

use crate::testing::{Activity, Behavior, FakeStage};
use crate::{
    FakeSleeper, JobKey, PipelineDeps, PipelineService, PipelineStage, PipelineTrigger,
    RecordedProgress, SqliteJobRepo,
};

/// The time every harness starts at.
pub(super) const NOW: &str = "2026-10-08T09:00:00Z";

/// A wired pipeline and the fakes around it.
pub(super) struct Harness {
    pub(super) service: Arc<PipelineService>,
    pub(super) lifecycle: Arc<MeetingLifecycleService>,
    pub(super) meetings: Arc<SqliteMeetingRepo>,
    pub(super) jobs: Arc<SqliteJobRepo>,
    pub(super) database: Database,
    pub(super) sleeper: Arc<FakeSleeper>,
    pub(super) progress: Arc<RecordedProgress>,
}

/// How the harness's sleeper behaves.
#[derive(Clone, Copy)]
pub(super) enum Waits {
    /// Every wait ends at once, moving the clock.
    Instant,
    /// Waits never end, so a test can interrupt one.
    Stalled,
}

impl Harness {
    /// A reconciled pipeline over a fresh database.
    pub(super) async fn new(stages: Vec<Arc<dyn PipelineStage>>) -> Self {
        Self::on(test_db().await, stages, Waits::Instant).await
    }

    /// A reconciled pipeline over `database`: a "restart" when the database
    /// already holds jobs.
    pub(super) async fn on(
        database: Database,
        stages: Vec<Arc<dyn PipelineStage>>,
        waits: Waits,
    ) -> Self {
        let harness = Self::unstarted(database, stages, waits);
        harness.service.reconcile().await.unwrap();
        harness
    }

    fn unstarted(database: Database, stages: Vec<Arc<dyn PipelineStage>>, waits: Waits) -> Self {
        let clock = Arc::new(FakeClock::new(at(NOW)));
        let meetings = Arc::new(SqliteMeetingRepo::new(database.clone()));
        let lifecycle = Arc::new(MeetingLifecycleService::new(meetings.clone(), clock.clone()));
        let sleeper = Arc::new(match waits {
            Waits::Instant => FakeSleeper::new(clock.clone()),
            Waits::Stalled => FakeSleeper::stalled(clock.clone()),
        });
        let progress = Arc::new(RecordedProgress::default());
        let jobs = Arc::new(SqliteJobRepo::new(database.clone()));
        let service = Arc::new(PipelineService::new(PipelineDeps {
            jobs: jobs.clone(),
            stages,
            lifecycle: lifecycle.clone(),
            meetings: meetings.clone(),
            clock,
            sleeper: sleeper.clone(),
            progress: progress.clone(),
        }));
        lifecycle.subscribe(Arc::new(PipelineTrigger::new(&service)));
        Self {
            service,
            lifecycle,
            meetings,
            jobs,
            database,
            sleeper,
            progress,
        }
    }

    /// Creates a meeting in `status`.
    pub(super) async fn meeting(&self, id: &str, status: MeetingStatus) -> MeetingId {
        let meeting = NewMeeting {
            id: MeetingId::new(id),
            title: MeetingTitle::parse("Standup").unwrap(),
            status,
            source: MeetingSource::Live,
            created_at: at(NOW),
            started_at: Some(at(NOW)),
            folder_path: None,
        };
        self.meetings.create(&meeting).await.unwrap();
        meeting.id
    }

    /// Records a meeting and stops it, which starts its pipeline.
    pub(super) async fn record(&self, id: &str) -> MeetingId {
        let meeting = self.meeting(id, MeetingStatus::Recording).await;
        self.lifecycle
            .transition(&meeting, deino_domain::MeetingEvent::StopRecording)
            .await
            .unwrap();
        meeting
    }

    /// The meeting's status.
    pub(super) async fn status_of(&self, meeting: &MeetingId) -> MeetingStatus {
        self.meetings.state(meeting).await.unwrap().unwrap().status
    }

    /// `(stage, status, attempts)` of the meeting's stages, in order.
    pub(super) async fn stages(&self, meeting: &MeetingId) -> Vec<(String, JobStatus, u32)> {
        let statuses = self.service.status(meeting).await.unwrap();
        statuses
            .into_iter()
            .map(|stage| (stage.stage.to_string(), stage.status, stage.attempts))
            .collect()
    }

    /// `(stage, status)` of every progress event, in order.
    pub(super) fn events(&self) -> Vec<(String, JobStatus)> {
        self.progress
            .events()
            .into_iter()
            .map(|event| (event.stage.to_string(), event.status))
            .collect()
    }

    /// Waits until `count` events reported `status`.
    pub(super) async fn wait_for(&self, status: JobStatus, count: usize) {
        self.progress
            .wait_for(|events| events.iter().filter(|event| event.status == status).count() >= count)
            .await;
    }

    /// `(attempt, outcome)` of every attempt of `stage`, oldest first.
    pub(super) async fn runs(&self, meeting: &MeetingId, stage: &str) -> Vec<(u32, Option<String>)> {
        sqlx::query_as(
            "SELECT attempt, outcome FROM job_runs WHERE meeting_id = ?1 AND stage = ?2 ORDER BY id",
        )
        .bind(meeting)
        .bind(stage)
        .fetch_all(self.database.reader())
        .await
        .unwrap()
    }
}

/// The key of `stage` of `meeting`.
pub(super) fn key(meeting: &MeetingId, stage: &str) -> JobKey {
    JobKey {
        meeting_id: meeting.clone(),
        stage: StageId::new(stage),
    }
}

/// A run outcome as `job_runs` stores it.
pub(super) fn outcome(attempt: u32, outcome: &str) -> (u32, Option<String>) {
    (attempt, Some(outcome.to_owned()))
}

/// A fake stage that records its runs in `activity`.
pub(super) fn stage(
    id: &'static str,
    order: u16,
    behavior: Behavior,
    activity: &Arc<Activity>,
) -> Arc<dyn PipelineStage> {
    FakeStage::new(id, order, behavior)
        .sharing(activity)
        .into_dyn()
}

/// The stages `activity` saw start, in order.
pub(super) fn started(activity: &Activity) -> Vec<String> {
    activity.starts().into_iter().map(|(stage, _)| stage).collect()
}
