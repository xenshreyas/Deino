//! The local steps (AUT-01): each implements [`Step`](super::Step) for one
//! [`ActionKind`](super::ActionKind).
//!
//! | Step | Action | Ports |
//! |---|---|---|
//! | [`InsightStep`] | generate an insight (AI-06) | insight definitions, runs and the generator |
//! | [`TagStep`] | add a tag (ORG-06) | [`TagApplier`] |
//! | [`FolderStep`] | add to a folder (ORG-05) | [`MeetingFiler`] |
//! | [`ExportStep`] | export a file (EXP-01) | [`MeetingSource`](super::MeetingSource), the vault |
//! | [`NotifyStep`] | macOS notification | the `Notifier` (AI-05's), meeting titles |
//! | [`ShellStep`] | run a shell script | [`ShellSwitch`], [`MeetingFolders`] |
//!
//! [`LocalStepPorts::registry`] registers all of them; provider steps
//! (INT-01..07, DEV-01) are added by their cards.

mod export;
mod insight;
mod notify;
mod organize;
mod shell;
#[cfg(test)]
mod shell_tests;

use std::sync::Arc;

use deino_domain::Clock;
pub use export::ExportStep;
pub use insight::InsightStep;
pub use notify::NotifyStep;
pub use organize::{FolderStep, MeetingFiler, TagApplier, TagStep};
pub use shell::{MeetingFolders, RecordingFolders, SHELL_TIMEOUT, ShellStep, ShellSwitch};

use super::{MeetingSource, StepRegistry};
use crate::action_items::Notifier;
use crate::export::Vault;
use crate::insights::{InsightGenerator, InsightRepo, InsightResultRepo, MeetingRecords};

/// What the local steps are built from; the composition root fills it.
#[derive(Clone)]
pub struct LocalStepPorts {
    /// Insight definitions.
    pub insight_defs: Arc<dyn InsightRepo>,
    /// Insight runs.
    pub insight_runs: Arc<dyn InsightResultRepo>,
    /// Runs insights (the `insights` stage's generator).
    pub generator: Arc<dyn InsightGenerator>,
    /// Applies tags.
    pub tags: Arc<dyn TagApplier>,
    /// Files meetings.
    pub filer: Arc<dyn MeetingFiler>,
    /// Reads meetings for exports and payloads.
    pub meetings: Arc<dyn MeetingSource>,
    /// Meeting titles for notifications.
    pub records: Arc<dyn MeetingRecords>,
    /// Writes exported files.
    pub vault: Arc<dyn Vault>,
    /// Shows notifications.
    pub notifier: Arc<dyn Notifier>,
    /// Settings → Automations → "Allow shell scripts".
    pub shell_switch: Arc<dyn ShellSwitch>,
    /// Each meeting's folder, a script's working directory.
    pub folders: Arc<dyn MeetingFolders>,
    /// When insight runs are asked for.
    pub clock: Arc<dyn Clock>,
}

impl LocalStepPorts {
    /// Every local step, registered.
    pub fn registry(&self) -> StepRegistry {
        StepRegistry::new()
            .with(Arc::new(InsightStep::new(self)))
            .with(Arc::new(TagStep::new(Arc::clone(&self.tags))))
            .with(Arc::new(FolderStep::new(Arc::clone(&self.filer))))
            .with(Arc::new(ExportStep::new(
                Arc::clone(&self.meetings),
                Arc::clone(&self.vault),
            )))
            .with(Arc::new(NotifyStep::new(
                Arc::clone(&self.notifier),
                Arc::clone(&self.records),
            )))
            .with(Arc::new(ShellStep::new(self)))
    }
}
