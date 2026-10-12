//! [`AutomationEngine`]: the `automations` pipeline stage's rules.
//!
//! For one meeting:
//!
//! 1. **Decide what to run.** Every automation that runs on its own and
//!    matches the meeting gets a `pipeline` run, unless it already has a
//!    run on the meeting: re-processing a meeting never repeats an
//!    automation (idempotent; the user's Rerun resets a run). Waiting runs
//!    (`pending` from Run or Rerun, `armed` by a tag) join them; an armed
//!    run whose automation no longer matches is dropped.
//! 2. **Run each run's steps in order** through the [`StepRegistry`],
//!    storing each step's result as it finishes. A failed step does not
//!    stop the rest unless the automation stops on failure (then they are
//!    `skipped`). Steps already done are not run again when the stage
//!    retries: their outputs feed the later steps as before.
//! 3. A failure that may pass (a rate-limited insight) leaves the step
//!    pending and fails the stage so the pipeline retries, unless this is
//!    the last attempt; then it is stored like any other failure.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{
    AutomationRunId, AutomationRunStatus, AutomationStepStatus, Clock, MeetingFacts, MeetingId,
};

use super::{
    Action, Automation, AutomationError, AutomationRepo, AutomationRun, FactsSource, RunRepo,
    RunRequest, StepContext, StepOutput, StepRegistry, StepResult, Trigger,
};

/// How a meeting's automations ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineOutcome {
    /// No automation applies, and none was asked for.
    NothingToRun,
    /// Runs ran; `failed` of them had a failed step.
    Ran {
        /// How many runs ran.
        runs: usize,
        /// How many of them failed.
        failed: usize,
    },
}

/// Runs a meeting's automations: the port the `automations` stage calls.
#[async_trait]
pub trait AutomationRunner: Send + Sync {
    /// Runs what `meeting` needs (see the module docs).
    async fn run_meeting(
        &self,
        meeting: &MeetingId,
        final_attempt: bool,
    ) -> Result<EngineOutcome, AutomationError>;
}

/// The ports the engine works through.
#[derive(Clone)]
pub struct EngineDeps {
    /// The automations.
    pub automations: Arc<dyn AutomationRepo>,
    /// Their runs.
    pub runs: Arc<dyn RunRepo>,
    /// What conditions read.
    pub facts: FactsSource,
    /// The steps.
    pub steps: StepRegistry,
    /// Run times and IDs.
    pub clock: Arc<dyn Clock>,
}

/// Runs automations on meetings.
pub struct AutomationEngine {
    deps: EngineDeps,
}

impl AutomationEngine {
    /// The engine over its ports.
    pub fn new(deps: EngineDeps) -> Self {
        Self { deps }
    }

    /// Adds a `pipeline` run for every matching automation without a run.
    async fn request_matching(
        &self,
        meeting: &MeetingId,
        facts: &MeetingFacts,
        automations: &[Automation],
    ) -> Result<(), AutomationError> {
        let now = self.deps.clock.now();
        for automation in automations.iter().filter(|a| a.applies_to(facts)) {
            let request = RunRequest {
                id: AutomationRunId::generate(self.deps.clock.as_ref()),
                automation_id: automation.id,
                meeting_id: meeting.clone(),
                trigger: Trigger::Pipeline,
                status: AutomationRunStatus::Pending,
                at: now,
            };
            self.deps.runs.request(&request, false).await?;
        }
        Ok(())
    }

    /// The oldest waiting run on `meeting` not in `seen`.
    async fn next_waiting(
        &self,
        meeting: &MeetingId,
        seen: &[AutomationRunId],
    ) -> Result<Option<AutomationRun>, AutomationError> {
        let runs = self.deps.runs.for_meeting(meeting).await?;
        Ok(runs
            .into_iter()
            .rev()
            .find(|run| run.status.is_waiting() && !seen.contains(&run.id)))
    }

    /// Runs one run's steps and stores how it ended; true when it failed.
    async fn execute(
        &self,
        automation: &Automation,
        run: &AutomationRun,
        final_attempt: bool,
    ) -> Result<bool, AutomationError> {
        let positions: Vec<_> = automation
            .steps
            .iter()
            .zip(0_u32..)
            .map(|(action, position)| (position, action.kind()))
            .collect();
        self.deps.runs.start_steps(&run.id, &positions).await?;
        let mut progress = Progress::resumed(run);
        for (action, position) in automation.steps.iter().zip(0_u32..) {
            if progress.finished(position) {
                continue;
            }
            let ctx = StepContext {
                automation,
                run_id: &run.id,
                meeting: &run.meeting_id,
                trigger: run.trigger,
                earlier: &progress.outputs,
                final_attempt,
            };
            let result = self.step(action, ctx, (position, progress.failed)).await?;
            self.deps.runs.save_step(&result).await?;
            progress.record(&result);
        }
        let status = if progress.failed {
            AutomationRunStatus::Failed
        } else {
            AutomationRunStatus::Done
        };
        self.deps
            .runs
            .finish(&run.id, status, self.deps.clock.now())
            .await?;
        Ok(progress.failed)
    }

    /// Runs (or skips, after an earlier failure when the automation stops
    /// on failure) one step; the result to store.
    async fn step(
        &self,
        action: &Action,
        ctx: StepContext<'_>,
        (position, failed_before): (u32, bool),
    ) -> Result<StepResult, AutomationError> {
        let mut result = StepResult {
            run_id: *ctx.run_id,
            position,
            status: AutomationStepStatus::Skipped,
            failure: None,
            output: None,
            at: self.deps.clock.now(),
        };
        if failed_before && ctx.automation.stop_on_failure {
            return Ok(result);
        }
        match self.deps.steps.run(action, ctx).await {
            Ok(output) => {
                result.status = AutomationStepStatus::Done;
                result.output = Some(output);
            }
            Err(error) if error.is_retryable() && !ctx.final_attempt => {
                return Err(AutomationError::Step(error));
            }
            Err(error) => {
                result.status = AutomationStepStatus::Failed;
                result.failure = Some((error.failure(), error.to_string()));
            }
        }
        result.at = self.deps.clock.now();
        Ok(result)
    }
}

/// Where a run stands while its steps run.
struct Progress {
    /// Steps already finished in an earlier attempt.
    finished: Vec<u32>,
    /// What the steps done so far produced, in order.
    outputs: Vec<StepOutput>,
    /// True once a step failed.
    failed: bool,
}

impl Progress {
    /// The progress stored on `run` by an earlier attempt.
    fn resumed(run: &AutomationRun) -> Self {
        let done = run
            .steps
            .iter()
            .filter(|step| step.status != AutomationStepStatus::Pending);
        Self {
            finished: done.clone().map(|step| step.position).collect(),
            outputs: done
                .clone()
                .filter_map(|step| step.output.clone())
                .collect(),
            failed: done
                .clone()
                .any(|step| step.status == AutomationStepStatus::Failed),
        }
    }

    fn finished(&self, position: u32) -> bool {
        self.finished.contains(&position)
    }

    /// Takes in a step that just finished.
    fn record(&mut self, result: &StepResult) {
        if let Some(output) = &result.output {
            self.outputs.push(output.clone());
        }
        self.failed |= result.status == AutomationStepStatus::Failed;
    }
}

#[async_trait]
impl AutomationRunner for AutomationEngine {
    async fn run_meeting(
        &self,
        meeting: &MeetingId,
        final_attempt: bool,
    ) -> Result<EngineOutcome, AutomationError> {
        let Some(facts) = self.deps.facts.facts(meeting).await? else {
            return Ok(EngineOutcome::NothingToRun);
        };
        let automations = self.deps.automations.list().await?;
        self.request_matching(meeting, &facts, &automations).await?;
        let mut seen: Vec<AutomationRunId> = Vec::new();
        let (mut runs, mut failed) = (0, 0);
        // A step may tag the meeting and arm another automation: run until
        // no run is waiting, each run at most once per stage attempt.
        while let Some(run) = self.next_waiting(meeting, &seen).await? {
            seen.push(run.id);
            let Some(automation) = automations.iter().find(|a| a.id == run.automation_id) else {
                continue;
            };
            if run.status == AutomationRunStatus::Armed && !automation.conditions.matches(&facts) {
                self.deps.runs.discard(&run.id).await?;
                continue;
            }
            runs += 1;
            failed += usize::from(self.execute(automation, &run, final_attempt).await?);
        }
        log::info!(target: "automations", "ran {runs} automations on meeting {meeting} ({failed} failed)");
        Ok(if runs == 0 {
            EngineOutcome::NothingToRun
        } else {
            EngineOutcome::Ran { runs, failed }
        })
    }
}
