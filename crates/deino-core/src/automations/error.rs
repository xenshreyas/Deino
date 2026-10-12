//! What can go wrong with automations: [`AutomationError`] for the service
//! and [`StepError`] for one step of a run. Messages name fields, IDs and
//! exit codes and never what a meeting said. A failed script's stderr tail
//! is kept in the run log (the database) for the user, and never logged.

use deino_db::DbError;
use deino_domain::{AutomationId, ErrorClass, ErrorCode, InvalidCondition, MeetingId};
use serde::{Deserialize, Serialize};

use crate::export::ExportError;
use crate::folders::FolderError;
use crate::insights::InsightError;
use crate::tags::TagError;
use crate::webhooks::WebhookError;

/// An automations failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AutomationError {
    /// No automation has the ID.
    #[error("automation {0} was not found")]
    NotFound(AutomationId),
    /// No meeting outside the trash has the ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// There is no processed meeting to test on.
    #[error("there is no finished meeting to test on")]
    NoMeeting,
    /// A field of the automation breaks a rule.
    #[error("{field} {rule}")]
    Invalid {
        /// The field, for example "name".
        field: &'static str,
        /// What is wrong, for example "cannot be empty".
        rule: &'static str,
    },
    /// A condition cannot match as written.
    #[error(transparent)]
    Condition(#[from] InvalidCondition),
    /// Another automation already has the name.
    #[error("an automation with this name already exists")]
    DuplicateName,
    /// The post-meeting pipeline would not take the run.
    #[error("could not schedule the run: {0}")]
    Schedule(String),
    /// A step failed in a way that may pass; the stage tries again.
    #[error(transparent)]
    Step(StepError),
    /// The database failed; the source carries the detail.
    #[error("could not load or save automations")]
    Storage(#[from] DbError),
}

impl AutomationError {
    /// A broken rule of `field`.
    pub fn invalid(field: &'static str, rule: &'static str) -> Self {
        Self::Invalid { field, rule }
    }

    /// True for failures that may pass if the stage tries again.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Step(error) => error.is_retryable(),
            Self::Storage(_) => true,
            _ => false,
        }
    }
}

impl ErrorClass for AutomationError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) | Self::MeetingNotFound(_) | Self::NoMeeting => ErrorCode::NotFound,
            Self::Invalid { .. } | Self::Condition(_) => ErrorCode::Invalid,
            Self::DuplicateName => ErrorCode::Conflict,
            Self::Schedule(_) | Self::Step(_) => ErrorCode::Unavailable,
            Self::Storage(error) => error.code(),
        }
    }
}

/// Why a step failed, stored with it (`automation_run_steps.error_code`) so
/// the run log can say what to do about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum StepFailure {
    /// Settings → Automations does not allow shell scripts.
    ShellDisabled,
    /// The script ran past its time limit and was stopped.
    Timeout,
    /// The script exited with a failure status.
    ExitStatus,
    /// The insight, tag or folder the step names is gone.
    MissingTarget,
    /// The step's kind is not built yet (a "WIP" provider step).
    Unavailable,
    /// The language model could not answer.
    Llm,
    /// Reading or writing files, or starting a process, failed.
    Io,
    /// A webhook was not delivered (DEV-01).
    Delivery,
    /// Deino's own storage failed.
    Storage,
}

/// One step's failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StepError {
    /// Shell scripts are off in Settings → Automations.
    #[error("shell scripts are turned off in Settings → Automations")]
    ShellDisabled,
    /// The script ran longer than allowed and its process group was killed.
    #[error("the script ran longer than {seconds} seconds and was stopped")]
    Timeout {
        /// The limit.
        seconds: u64,
    },
    /// The script exited unsuccessfully; `code` is `None` when a signal
    /// ended it.
    #[error("the script exited with status {}{}", code.map_or_else(|| "killed".to_owned(), |code| code.to_string()), stderr_suffix(stderr))]
    ExitStatus {
        /// The exit code.
        code: Option<i32>,
        /// The end of what it printed on stderr, for the run log only.
        stderr: String,
    },
    /// What the step names no longer exists.
    #[error("the {0} this step uses no longer exists")]
    MissingTarget(&'static str),
    /// No step runs this kind of action yet.
    #[error("this step is not available yet")]
    Unavailable,
    /// The insight could not be generated.
    #[error(transparent)]
    Insight(#[from] InsightError),
    /// The insight ran and its run failed; the run's user-safe reason.
    #[error("the insight failed: {0}")]
    InsightFailed(String),
    /// The tag could not be applied.
    #[error(transparent)]
    Tag(#[from] TagError),
    /// The meeting could not be filed.
    #[error(transparent)]
    Folder(#[from] FolderError),
    /// The file could not be written.
    #[error(transparent)]
    Export(#[from] ExportError),
    /// A file or process operation failed.
    #[error("could not {0}")]
    Io(&'static str, #[source] std::io::Error),
    /// The webhook could not be delivered (DEV-01).
    #[error(transparent)]
    Webhook(#[from] WebhookError),
    /// Deino's storage failed.
    #[error("could not load or save the run")]
    Storage(#[from] DbError),
}

impl StepError {
    /// The stored kind of failure.
    pub fn failure(&self) -> StepFailure {
        match self {
            Self::ShellDisabled => StepFailure::ShellDisabled,
            Self::Timeout { .. } => StepFailure::Timeout,
            Self::ExitStatus { .. } => StepFailure::ExitStatus,
            Self::MissingTarget(_) | Self::Webhook(WebhookError::NotFound(_)) => {
                StepFailure::MissingTarget
            }
            Self::Unavailable => StepFailure::Unavailable,
            Self::Insight(InsightError::Llm(_)) | Self::InsightFailed(_) => StepFailure::Llm,
            Self::Io(..) | Self::Export(_) => StepFailure::Io,
            Self::Webhook(_) => StepFailure::Delivery,
            Self::Insight(_) | Self::Tag(_) | Self::Folder(_) | Self::Storage(_) => {
                StepFailure::Storage
            }
        }
    }

    /// True for failures that may pass if the stage tries again (a rate
    /// limit, a busy database); never for scripts, which are not re-run.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Insight(error) => error.is_retryable(),
            Self::Storage(_) => true,
            _ => false,
        }
    }
}

/// ": <stderr>" when the script printed an error, else nothing.
fn stderr_suffix(stderr: &str) -> String {
    if stderr.is_empty() {
        String::new()
    } else {
        format!(": {stderr}")
    }
}
