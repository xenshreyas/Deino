//! The step picker's catalogue: every [`ActionKind`] with its label, and
//! whether it can be added yet. Provider steps are planned and shown as
//! "WIP" (PARITY_PLAN D12) until their card registers a [`Step`](super::Step).

use serde::Serialize;

use super::{ActionKind, StepRegistry};

/// Whether the builder can add a kind of step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StepAvailability {
    /// Built and registered.
    Ready,
    /// Planned: listed, disabled, badged "WIP".
    Wip,
}

/// One entry of the step picker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StepInfo {
    /// The kind of step.
    pub kind: ActionKind,
    /// Its name in the picker.
    pub label: &'static str,
    /// One line on what it does.
    pub description: &'static str,
    /// Whether it can be added.
    pub availability: StepAvailability,
}

/// Every kind of step in picker order, with what it does.
const ENTRIES: [(ActionKind, &str, &str); 13] = [
    (
        ActionKind::GenerateInsight,
        "Generate insight",
        "Run a custom insight; later steps get its answers.",
    ),
    (ActionKind::AddTag, "Add tag", "Tag the meeting."),
    (
        ActionKind::AddToFolder,
        "Add to folder",
        "File the meeting in a folder.",
    ),
    (
        ActionKind::ExportFile,
        "Export file",
        "Write the meeting as a file into a folder.",
    ),
    (
        ActionKind::Notify,
        "Notification",
        "Show a macOS notification.",
    ),
    (
        ActionKind::RunShell,
        "Run shell script",
        "Run a command with the meeting as JSON on stdin.",
    ),
    (ActionKind::Webhook, "Webhook", "Send the meeting to a URL."),
    (ActionKind::Slack, "Slack", "Post the notes to a channel."),
    (
        ActionKind::Notion,
        "Notion",
        "Create a page or database row.",
    ),
    (
        ActionKind::Tasks,
        "Task tools",
        "Create tasks in Linear, Todoist and others.",
    ),
    (
        ActionKind::Crm,
        "CRM",
        "Update HubSpot, Salesforce or Attio.",
    ),
    (
        ActionKind::Document,
        "Google Docs or Apple Notes",
        "Create a document.",
    ),
    (ActionKind::Email, "Email", "Draft or send a recap email."),
];

/// The picker's entries: ready when `registry` runs the kind, else WIP.
pub fn catalog(registry: &StepRegistry) -> Vec<StepInfo> {
    let ready = registry.kinds();
    ENTRIES
        .iter()
        .map(|&(kind, label, description)| StepInfo {
            kind,
            label,
            description,
            availability: if ready.contains(&kind) {
                StepAvailability::Ready
            } else {
                StepAvailability::Wip
            },
        })
        .collect()
}
