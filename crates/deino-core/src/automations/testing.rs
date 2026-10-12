//! Fakes for automations' ports, for this crate's tests and the app's
//! contract tests (`test-support`).

use std::sync::Mutex;

use async_trait::async_trait;
use deino_domain::{AppError, MeetingId};

use super::steps::ShellSwitch;
use super::{Action, ActionKind, AutomationScheduler, Step, StepContext, StepError, StepOutput};
use crate::action_items::{Notice, Notifier};
use crate::testing::lock;

/// An [`AutomationScheduler`] that records the meetings it was asked for.
#[derive(Debug, Default)]
pub struct RecordedAutomationScheduler {
    scheduled: Mutex<Vec<MeetingId>>,
}

impl RecordedAutomationScheduler {
    /// The meetings scheduled, in order.
    pub fn scheduled(&self) -> Vec<MeetingId> {
        lock(&self.scheduled).clone()
    }
}

#[async_trait]
impl AutomationScheduler for RecordedAutomationScheduler {
    async fn schedule(&self, meeting: &MeetingId) -> Result<(), AppError> {
        lock(&self.scheduled).push(meeting.clone());
        Ok(())
    }
}

/// What a [`ScriptedStep`] answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    /// Succeeds with [`StepOutput::Notified`].
    Succeed,
    /// Fails for good.
    Fail,
    /// Fails in a way that may pass (a busy database).
    FailRetryably,
}

/// A [`Step`] of any kind that answers as scripted and records its calls.
pub struct ScriptedStep {
    kind: ActionKind,
    script: Mutex<Vec<Script>>,
    calls: Mutex<Vec<(MeetingId, usize)>>,
}

impl ScriptedStep {
    /// A step of `kind` answering each call with the next of `script`, then
    /// succeeding.
    pub fn new(kind: ActionKind, script: Vec<Script>) -> Self {
        Self {
            kind,
            script: Mutex::new(script),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Each call's meeting and how many earlier outputs it was given.
    pub fn calls(&self) -> Vec<(MeetingId, usize)> {
        lock(&self.calls).clone()
    }
}

#[async_trait]
impl Step for ScriptedStep {
    fn kind(&self) -> ActionKind {
        self.kind
    }

    async fn run(&self, _action: &Action, ctx: StepContext<'_>) -> Result<StepOutput, StepError> {
        lock(&self.calls).push((ctx.meeting.clone(), ctx.earlier.len()));
        let next = {
            let mut script = lock(&self.script);
            if script.is_empty() {
                Script::Succeed
            } else {
                script.remove(0)
            }
        };
        match next {
            Script::Succeed => Ok(StepOutput::Notified),
            Script::Fail => Err(StepError::MissingTarget("tag")),
            Script::FailRetryably => Err(StepError::Storage(deino_db::DbError::Query(
                sqlx::Error::PoolTimedOut,
            ))),
        }
    }
}

/// A [`ShellSwitch`] fixed on or off.
#[derive(Clone, Copy, Debug)]
pub struct FixedShellSwitch(pub bool);

#[async_trait]
impl ShellSwitch for FixedShellSwitch {
    async fn shell_allowed(&self) -> bool {
        self.0
    }
}

/// A [`Notifier`] that records what it showed.
#[derive(Debug, Default)]
pub struct RecordedNotices {
    shown: Mutex<Vec<Notice>>,
}

impl RecordedNotices {
    /// What was shown, in order.
    pub fn shown(&self) -> Vec<Notice> {
        lock(&self.shown).clone()
    }
}

impl Notifier for RecordedNotices {
    fn notify(&self, notice: &Notice) {
        lock(&self.shown).push(notice.clone());
    }
}
