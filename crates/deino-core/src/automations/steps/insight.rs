//! [`InsightStep`]: runs a custom insight (AI-06) on the meeting and hands
//! its answers to the later steps.
//!
//! The `insights` stage (85) runs before `automations` (95), so an
//! automatic insight is usually answered already: a done run for the
//! insight's current revision is reused as is. Otherwise the step asks for
//! a run and lets the insights generator answer it now, through the same
//! rules (and the same `insight_results` row) as the Insights tab.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{Clock, InsightId, InsightStatus, MeetingId};

use super::LocalStepPorts;
use crate::automations::{Action, ActionKind, Step, StepContext, StepError, StepOutput};
use crate::insights::{
    Attempt, InsightDef, InsightGenerator, InsightRepo, InsightResultRepo, InsightRun,
};

/// Runs [`Action::GenerateInsight`].
pub struct InsightStep {
    defs: Arc<dyn InsightRepo>,
    runs: Arc<dyn InsightResultRepo>,
    generator: Arc<dyn InsightGenerator>,
    clock: Arc<dyn Clock>,
}

impl InsightStep {
    /// The step over the insight ports.
    pub fn new(ports: &LocalStepPorts) -> Self {
        Self {
            defs: Arc::clone(&ports.insight_defs),
            runs: Arc::clone(&ports.insight_runs),
            generator: Arc::clone(&ports.generator),
            clock: Arc::clone(&ports.clock),
        }
    }

    /// The insight's latest run on `meeting`.
    async fn latest(
        &self,
        meeting: &MeetingId,
        insight: &InsightId,
    ) -> Result<Option<InsightRun>, StepError> {
        let runs = self.runs.for_meeting(meeting).await?;
        Ok(runs.into_iter().find(|run| run.insight_id == *insight))
    }

    /// Asks for a run and has the generator answer it now.
    async fn generate(&self, def: &InsightDef, ctx: StepContext<'_>) -> Result<(), StepError> {
        let meetings = std::slice::from_ref(ctx.meeting);
        self.runs
            .request(&def.id, meetings, self.clock.now())
            .await?;
        let attempt = if ctx.final_attempt {
            Attempt::Last
        } else {
            Attempt::Retryable
        };
        self.generator.generate(ctx.meeting, attempt).await?;
        Ok(())
    }
}

/// True when `run` holds answers to the definition's current question.
fn is_fresh(run: Option<&InsightRun>, def: &InsightDef) -> bool {
    run.is_some_and(|run| run.status == InsightStatus::Done && !run.is_stale(def.revision))
}

#[async_trait]
impl Step for InsightStep {
    fn kind(&self) -> ActionKind {
        ActionKind::GenerateInsight
    }

    async fn run(&self, action: &Action, ctx: StepContext<'_>) -> Result<StepOutput, StepError> {
        let Action::GenerateInsight { insight_id } = action else {
            return Err(StepError::Unavailable);
        };
        let def = self
            .defs
            .get(insight_id)
            .await?
            .ok_or(StepError::MissingTarget("insight"))?;
        let mut run = self.latest(ctx.meeting, insight_id).await?;
        if !is_fresh(run.as_ref(), &def) {
            self.generate(&def, ctx).await?;
            run = self.latest(ctx.meeting, insight_id).await?;
        }
        match run {
            Some(run) if run.status == InsightStatus::Done => Ok(StepOutput::Insight {
                insight_id: def.id,
                name: def.name,
                answers: run.answers,
            }),
            Some(run) => Err(StepError::InsightFailed(
                run.error.unwrap_or_else(|| "it did not finish".to_owned()),
            )),
            None => Err(StepError::InsightFailed("it did not run".to_owned())),
        }
    }
}
