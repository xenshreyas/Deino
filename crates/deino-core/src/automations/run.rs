//! Runs: an automation's latest run on a meeting, why it started, and what
//! each of its steps did.

use chrono::{DateTime, Utc};
use deino_domain::{
    AutomationId, AutomationRunId, AutomationRunStatus, AutomationStepStatus, AutomationTrigger,
    FolderId, InsightId, MeetingId, TagId,
};
use serde::{Deserialize, Serialize};

use super::{ActionKind, StepFailure};
use crate::insights::InsightAnswer;

/// Why a run started.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Trigger {
    /// The meeting finished processing.
    Pipeline,
    /// A tag the automation's conditions test for was applied.
    TagApplied {
        /// The tag.
        #[serde(rename = "tagId")]
        tag_id: TagId,
    },
    /// The user ran it.
    Manual,
}

impl Trigger {
    /// The stored kind (`automation_runs.trigger`).
    pub const fn kind(self) -> AutomationTrigger {
        match self {
            Self::Pipeline => AutomationTrigger::Pipeline,
            Self::TagApplied { .. } => AutomationTrigger::TagApplied,
            Self::Manual => AutomationTrigger::Manual,
        }
    }

    /// The tag of a tag-applied run.
    pub const fn tag(self) -> Option<TagId> {
        match self {
            Self::TagApplied { tag_id } => Some(tag_id),
            Self::Pipeline | Self::Manual => None,
        }
    }

    /// The trigger stored as `kind` with `tag`; a tag-applied run whose
    /// tag was deleted reads as manual.
    pub const fn stored(kind: AutomationTrigger, tag: Option<TagId>) -> Self {
        match (kind, tag) {
            (AutomationTrigger::Pipeline, _) => Self::Pipeline,
            (AutomationTrigger::TagApplied, Some(tag_id)) => Self::TagApplied { tag_id },
            (AutomationTrigger::TagApplied | AutomationTrigger::Manual, _) => Self::Manual,
        }
    }
}

/// What a step produced, for the run log and for later steps' payloads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepOutput {
    /// An insight's answers.
    Insight {
        /// The insight.
        #[serde(rename = "insightId")]
        insight_id: InsightId,
        /// Its name.
        name: String,
        /// Its answers on the meeting.
        answers: Vec<InsightAnswer>,
    },
    /// A tag was applied (`added` is false when the meeting had it).
    Tagged {
        /// The tag.
        #[serde(rename = "tagId")]
        tag_id: TagId,
        /// Whether it was new on the meeting.
        added: bool,
    },
    /// The meeting was filed.
    Filed {
        /// The folder.
        #[serde(rename = "folderId")]
        folder_id: FolderId,
    },
    /// A file was written.
    Exported {
        /// Its full path.
        path: String,
    },
    /// A notification was shown.
    Notified,
    /// A script ran and exited successfully.
    Script {
        /// The last lines it printed, at most a few hundred characters.
        output: String,
    },
}

/// One step of a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RunStep {
    /// Its index in the automation's steps when the run started.
    pub position: u32,
    /// What kind of step it was.
    pub kind: ActionKind,
    /// Pending, done, failed or skipped.
    pub status: AutomationStepStatus,
    /// Why it failed.
    pub failure: Option<StepFailure>,
    /// Why it failed, user-safe.
    pub error: Option<String>,
    /// What it produced.
    pub output: Option<StepOutput>,
    /// When it finished.
    pub finished_at: Option<DateTime<Utc>>,
}

/// An automation's latest run on a meeting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRun {
    /// The run.
    pub id: AutomationRunId,
    /// The automation.
    pub automation_id: AutomationId,
    /// The meeting.
    pub meeting_id: MeetingId,
    /// Why it started.
    pub trigger: Trigger,
    /// Where it stands.
    pub status: AutomationRunStatus,
    /// Its steps, in order; empty until the stage starts it.
    pub steps: Vec<RunStep>,
    /// When it was asked for (or armed).
    pub requested_at: DateTime<Utc>,
    /// When its last step finished.
    pub finished_at: Option<DateTime<Utc>>,
}

/// A run to store: asked for, armed or started by the pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRequest {
    /// The run's ID (kept when an existing run is replaced).
    pub id: AutomationRunId,
    /// The automation.
    pub automation_id: AutomationId,
    /// The meeting.
    pub meeting_id: MeetingId,
    /// Why.
    pub trigger: Trigger,
    /// `Armed` or `Pending`.
    pub status: AutomationRunStatus,
    /// When.
    pub at: DateTime<Utc>,
}

/// One step's result, to store.
#[derive(Clone, Debug, PartialEq)]
pub struct StepResult {
    /// The run.
    pub run_id: AutomationRunId,
    /// The step.
    pub position: u32,
    /// Done, failed or skipped.
    pub status: AutomationStepStatus,
    /// Why it failed, with a user-safe sentence.
    pub failure: Option<(StepFailure, String)>,
    /// What it produced.
    pub output: Option<StepOutput>,
    /// When it finished.
    pub at: DateTime<Utc>,
}
