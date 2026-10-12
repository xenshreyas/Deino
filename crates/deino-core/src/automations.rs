//! Automations (AUT-01, ADR 0042): "when a meeting like this finishes, do
//! these steps", Circleback's automations made local.
//!
//! An [`Automation`] is a name, switches (enabled, run automatically, stop
//! on failure), conditions (`deino_domain::ConditionGroups`, shared with
//! AI-06: AND within a group, OR across groups, none for every meeting) and
//! an ordered list of [`Action`]s. Each action is run by the [`Step`] of
//! its [`ActionKind`], found in the [`StepRegistry`]; the local steps are in
//! `steps/`, and provider steps (Slack, Notion, CRMs, webhooks) are listed
//! as "WIP" until their cards register them (PARITY_PLAN D12).
//!
//! | File | Role |
//! |---|---|
//! | `action.rs`, `model.rs` | [`Action`], [`ActionKind`], [`PayloadFields`]; [`Automation`], [`AutomationInput`] and their rules |
//! | `step.rs`, `catalog.rs` | the [`Step`] trait, [`StepContext`], [`StepRegistry`]; the picker's [`StepInfo`] |
//! | `steps.rs`, `steps/` | the local steps and [`LocalStepPorts`] |
//! | `payload.rs` | [`AutomationPayload`], the JSON a script (or a webhook, later) receives |
//! | `run.rs` | [`AutomationRun`], [`RunStep`], [`StepOutput`], [`Trigger`] |
//! | `engine.rs` | [`AutomationEngine`], the `automations` pipeline stage's rules |
//! | `service.rs`, `triggers.rs`, `views.rs` | [`AutomationService`]: definitions, manual and tag-triggered runs, the meeting menu, matching meetings |
//! | `facts.rs` | [`FactsSource`]: AI-06's facts loader, judged against the user's domains |
//! | `repo.rs`, `runs_repo.rs`, `rows.rs` | [`AutomationRepo`], [`RunRepo`] on [`SqliteAutomationRepo`] |
//!
//! Triggers: the `automations` stage (95) after a meeting is processed;
//! a tag applied to a meeting ([`AutomationService::tag_applied`], which
//! arms runs on meetings not processed yet); and the user's runs.

mod action;
mod catalog;
mod engine;
#[cfg(test)]
mod engine_tests;
mod error;
mod facts;
#[cfg(test)]
mod harness;
mod model;
mod payload;
mod repo;
#[cfg(test)]
mod repo_tests;
mod rows;
mod run;
mod runs_repo;
mod service;
#[cfg(test)]
mod service_tests;
mod step;
pub mod steps;
#[cfg(any(test, feature = "test-support"))]
mod testing;
mod triggers;
mod views;

pub use action::{Action, ActionKind, MAX_COMMAND_CHARS, MAX_MESSAGE_CHARS, PayloadFields};
pub use catalog::{StepAvailability, StepInfo, catalog};
pub use engine::{AutomationEngine, AutomationRunner, EngineDeps, EngineOutcome};
pub use error::{AutomationError, StepError, StepFailure};
pub use facts::FactsSource;
pub use model::{Automation, AutomationInput, MAX_NAME_CHARS, MAX_STEPS};
pub use payload::{
    AssembledMeetings, AutomationPayload, MeetingSource, PAYLOAD_FORMAT, PAYLOAD_VERSION,
    PayloadAutomation, PayloadMeeting, build_payload,
};
pub use repo::{AutomationRepo, SqliteAutomationRepo};
pub use run::{AutomationRun, RunRequest, RunStep, StepOutput, StepResult, Trigger};
pub use runs_repo::RunRepo;
pub use service::{AutomationPorts, AutomationScheduler, AutomationService};
pub use step::{Step, StepContext, StepRegistry};
pub use steps::LocalStepPorts;
#[cfg(any(test, feature = "test-support"))]
pub use testing::{
    FixedShellSwitch, RecordedAutomationScheduler, RecordedNotices, Script, ScriptedStep,
};
pub use triggers::MAX_RUN_MEETINGS;
pub use views::{MatchingMeeting, MeetingAutomation};
