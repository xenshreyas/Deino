//! The pipeline's API: schedule, re-run, inspect and cancel a meeting's
//! pipeline, and start and stop the runner.
//!
//! Scheduling writes the jobs first and only then wakes the runner, so
//! everything the runner does can be resumed from the database.

use std::sync::Arc;

use deino_core::meetings::{MeetingLifecycleRepo, MeetingLifecycleService};
use deino_domain::{Clock, JobStatus, MeetingEvent, MeetingId, MeetingStatus, StageId};

use crate::plan;
use crate::runner::{Runner, RunnerDeps};
use crate::{
    Job, JobRepo, PipelineError, PipelineProgress, PipelineStage, ProgressSink, Registry,
    RetryPolicy, Sleeper, StageStatus,
};

/// What the pipeline is built from; the composition root fills it.
pub struct PipelineDeps {
    /// The queue.
    pub jobs: Arc<dyn JobRepo>,
    /// The stages to run, usually `registry::stages(&ports)`.
    pub stages: Vec<Arc<dyn PipelineStage>>,
    /// Moves meetings to `ready` or `failed`, and back to `processing`.
    pub lifecycle: Arc<MeetingLifecycleService>,
    /// Reads meeting status; lists `processing` meetings at startup.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// The injected clock.
    pub clock: Arc<dyn Clock>,
    /// Waits out retry backoff.
    pub sleeper: Arc<dyn Sleeper>,
    /// Where progress events go.
    pub progress: Arc<dyn ProgressSink>,
}

/// The post-meeting pipeline.
pub struct PipelineService {
    pub(crate) jobs: Arc<dyn JobRepo>,
    pub(crate) registry: Arc<Registry>,
    pub(crate) meetings: Arc<dyn MeetingLifecycleRepo>,
    lifecycle: Arc<MeetingLifecycleService>,
    pub(crate) clock: Arc<dyn Clock>,
    progress: Arc<dyn ProgressSink>,
    pub(crate) runner: Runner,
}

impl PipelineService {
    /// Builds the pipeline. Nothing runs until [`PipelineService::start`]
    /// (or [`PipelineService::reconcile`]) has reconciled the queue.
    pub fn new(deps: PipelineDeps) -> Self {
        let registry = Arc::new(Registry::new(deps.stages));
        let runner = Runner::new(RunnerDeps {
            jobs: Arc::clone(&deps.jobs),
            registry: Arc::clone(&registry),
            lifecycle: Arc::clone(&deps.lifecycle),
            clock: Arc::clone(&deps.clock),
            sleeper: deps.sleeper,
            progress: Arc::clone(&deps.progress),
            policy: RetryPolicy::DEFAULT,
        });
        Self {
            jobs: deps.jobs,
            registry,
            meetings: deps.meetings,
            lifecycle: deps.lifecycle,
            clock: deps.clock,
            progress: deps.progress,
            runner,
        }
    }

    /// Schedules every stage for `meeting` and wakes the runner, unless the
    /// meeting already has work queued or running. Returns whether it
    /// scheduled. The trigger calls it when a meeting enters `processing`.
    pub async fn enqueue(&self, meeting: &MeetingId) -> Result<bool, PipelineError> {
        let jobs = self.ordered_jobs(meeting).await?;
        let schedule = !plan::is_scheduled(&jobs);
        if schedule {
            self.schedule(meeting, &self.registry.ids()).await?;
        }
        self.runner.wake(meeting);
        Ok(schedule)
    }

    /// Runs `stage` of `meeting` again, or every stage when `stage` is
    /// `None`, and returns the meeting's stages. A `ready` or `failed`
    /// meeting moves back to `processing` while they run.
    pub async fn rerun(
        &self,
        meeting: &MeetingId,
        stage: Option<&StageId>,
    ) -> Result<Vec<StageStatus>, PipelineError> {
        let stages = match stage {
            Some(stage) => vec![self.registered(stage)?],
            None => self.registry.ids(),
        };
        let status = self.processable_status(meeting).await?;
        // Jobs first: entering `processing` triggers `enqueue`, which then
        // finds them scheduled and leaves the rest of the pipeline alone.
        self.schedule(meeting, &stages).await?;
        let reopened = match status {
            MeetingStatus::Ready | MeetingStatus::Failed => self
                .lifecycle
                .transition(meeting, MeetingEvent::Reprocess)
                .await
                .map(|_| ()),
            _ => Ok(()),
        };
        self.runner.wake(meeting);
        reopened?;
        self.status(meeting).await
    }

    /// The meeting's stages in the order they run, with their status. Empty
    /// when the pipeline never ran for the meeting.
    pub async fn status(&self, meeting: &MeetingId) -> Result<Vec<StageStatus>, PipelineError> {
        let jobs = self.ordered_jobs(meeting).await?;
        Ok(jobs.into_iter().map(StageStatus::from).collect())
    }

    /// Cancels the meeting's running or waiting stage; the stages after it
    /// stay pending until it is re-run. Returns `false` when nothing ran.
    pub fn cancel(&self, meeting: &MeetingId) -> bool {
        self.runner.cancel(meeting)
    }

    /// Reconciles the queue in the background, then lets the runner work.
    pub fn start(self: &Arc<Self>) {
        let service = Arc::clone(self);
        self.runner.spawn(async move {
            if let Err(error) = service.reconcile().await {
                let error = crate::error::ErrorChain(&error);
                log::error!(target: "pipeline", "could not resume the pipeline: {error}");
            }
        });
    }

    /// Stops every stage (interrupted jobs resume at the next start) and
    /// waits for the runner's tasks to end. Call it before the app exits.
    pub async fn shutdown(&self) {
        self.runner.shutdown().await;
    }

    /// Waits until the runner has nothing left to do (tests).
    #[cfg(any(test, feature = "test-support"))]
    pub async fn until_idle(&self) {
        self.runner.until_idle().await;
    }

    /// The meeting's jobs of registered stages, in stage order.
    async fn ordered_jobs(&self, meeting: &MeetingId) -> Result<Vec<Job>, PipelineError> {
        Ok(self.registry.in_order(self.jobs.jobs(meeting).await?))
    }

    /// Makes `stages` fresh pending jobs of `meeting` and reports them.
    async fn schedule(&self, meeting: &MeetingId, stages: &[StageId]) -> Result<(), PipelineError> {
        self.jobs
            .schedule(meeting, stages, self.clock.now())
            .await?;
        for stage in stages {
            self.progress.progress(&PipelineProgress {
                meeting_id: meeting.clone(),
                stage: stage.clone(),
                status: JobStatus::Pending,
            });
        }
        Ok(())
    }

    fn registered(&self, stage: &StageId) -> Result<StageId, PipelineError> {
        self.registry
            .find(stage)
            .map(|stage| stage.id())
            .ok_or_else(|| PipelineError::UnknownStage(stage.clone()))
    }

    /// The meeting's status, if its recording is done so it can be processed.
    async fn processable_status(
        &self,
        meeting: &MeetingId,
    ) -> Result<MeetingStatus, PipelineError> {
        let state = self.meetings.state(meeting).await?;
        let status = state
            .ok_or_else(|| PipelineError::MeetingNotFound(meeting.clone()))?
            .status;
        match status {
            MeetingStatus::Scheduled | MeetingStatus::Recording => {
                Err(PipelineError::NotRecorded {
                    meeting: meeting.clone(),
                    status,
                })
            }
            _ => Ok(status),
        }
    }
}
