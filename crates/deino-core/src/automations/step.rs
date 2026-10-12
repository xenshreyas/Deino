//! The [`Step`] trait every kind of automation step implements, the
//! [`StepContext`] a step runs in, and the [`StepRegistry`] that finds the
//! step for an action.
//!
//! **Adding a kind of step** (Open/Closed): an [`Action`] variant, a type
//! implementing [`Step`] for its [`ActionKind`], and one `with` call where
//! the registry is built. The engine never names a step.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{AutomationRunId, MeetingId};

use super::{Action, ActionKind, Automation, StepError, StepOutput, Trigger};

/// Where a step runs: the automation, the meeting and what earlier steps
/// of the same run produced.
#[derive(Clone, Copy, Debug)]
pub struct StepContext<'a> {
    /// The automation running.
    pub automation: &'a Automation,
    /// The run.
    pub run_id: &'a AutomationRunId,
    /// The meeting it runs on.
    pub meeting: &'a MeetingId,
    /// Why the run started.
    pub trigger: Trigger,
    /// What the earlier steps produced, in order (failed steps left out).
    pub earlier: &'a [StepOutput],
    /// True on the pipeline's last attempt: a failure that may pass is
    /// stored instead of retried.
    pub final_attempt: bool,
}

/// One kind of step.
///
/// Contract: [`Step::run`] is called only with actions of
/// [`Step::kind`]; any other action fails with [`StepError::Unavailable`].
/// A step is safe to run again on the same meeting (tagging twice is one
/// tag, exporting twice overwrites the file).
#[async_trait]
pub trait Step: Send + Sync {
    /// The kind of action it runs.
    fn kind(&self) -> ActionKind;

    /// Runs `action` on the context's meeting.
    async fn run(&self, action: &Action, ctx: StepContext<'_>) -> Result<StepOutput, StepError>;
}

/// The steps the engine can run, by kind.
#[derive(Clone, Default)]
pub struct StepRegistry {
    steps: Vec<Arc<dyn Step>>,
}

impl StepRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// The registry with `step` added; a kind registered twice keeps the
    /// first (and logs the second).
    #[must_use]
    pub fn with(mut self, step: Arc<dyn Step>) -> Self {
        if self.find(step.kind()).is_some() {
            log::error!(target: "automations", "step {} is registered twice; keeping the first", step.kind().as_str());
            return self;
        }
        self.steps.push(step);
        self
    }

    /// The step that runs `kind`.
    pub fn find(&self, kind: ActionKind) -> Option<&Arc<dyn Step>> {
        self.steps.iter().find(|step| step.kind() == kind)
    }

    /// Runs `action` with its registered step.
    pub async fn run(
        &self,
        action: &Action,
        ctx: StepContext<'_>,
    ) -> Result<StepOutput, StepError> {
        let Some(step) = self.find(action.kind()) else {
            return Err(StepError::Unavailable);
        };
        step.run(action, ctx).await
    }

    /// Every registered kind.
    pub fn kinds(&self) -> Vec<ActionKind> {
        self.steps.iter().map(|step| step.kind()).collect()
    }
}
