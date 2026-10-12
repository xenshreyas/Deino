//! [`Action`]: one step of an automation, as the user configured it, and
//! [`ActionKind`], every kind of step the builder knows, built or not.

use std::path::Path;

use deino_domain::{FolderId, InsightId, TagId, WebhookId};
use deino_export::ExportFormat;
use serde::{Deserialize, Serialize};

use super::AutomationError;

/// The longest shell command, in characters.
pub const MAX_COMMAND_CHARS: usize = 4_000;
/// The longest notification message, in characters.
pub const MAX_MESSAGE_CHARS: usize = 200;

/// One configured step. Each variant is run by the [`Step`](super::Step)
/// of the same [`ActionKind`]; a provider card (INT-01..07, DEV-01) adds
/// its variant here and its step to the registry.
///
/// Stored inside `automations.steps_json`: rename only with a migration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Runs a custom insight (AI-06) on the meeting; later steps receive
    /// its answers.
    GenerateInsight {
        /// The insight.
        #[serde(rename = "insightId")]
        insight_id: InsightId,
    },
    /// Puts a tag on the meeting, applied by "automation".
    AddTag {
        /// The tag.
        #[serde(rename = "tagId")]
        tag_id: TagId,
    },
    /// Files the meeting in a folder.
    AddToFolder {
        /// The folder.
        #[serde(rename = "folderId")]
        folder_id: FolderId,
    },
    /// Writes the meeting as a file (EXP-01) into a folder on the Mac.
    ExportFile {
        /// The file format.
        format: ExportFormat,
        /// The absolute path of the folder to write into.
        folder: String,
    },
    /// Shows a macOS notification.
    Notify {
        /// The notification's text; empty says the automation ran.
        message: String,
    },
    /// Runs a shell command with the meeting as JSON on stdin (Settings →
    /// Automations must allow shell scripts).
    RunShell {
        /// The command, run with `/bin/sh -c`.
        command: String,
        /// What the JSON payload carries.
        fields: PayloadFields,
    },
    /// Posts the meeting as signed JSON to a webhook endpoint (DEV-01) set
    /// up in Settings → Developer → Webhooks.
    Webhook {
        /// The endpoint.
        #[serde(rename = "webhookId")]
        webhook_id: WebhookId,
        /// What the JSON payload carries.
        fields: PayloadFields,
    },
}

impl Action {
    /// The kind of step that runs this action.
    pub const fn kind(&self) -> ActionKind {
        match self {
            Self::GenerateInsight { .. } => ActionKind::GenerateInsight,
            Self::AddTag { .. } => ActionKind::AddTag,
            Self::AddToFolder { .. } => ActionKind::AddToFolder,
            Self::ExportFile { .. } => ActionKind::ExportFile,
            Self::Notify { .. } => ActionKind::Notify,
            Self::RunShell { .. } => ActionKind::RunShell,
            Self::Webhook { .. } => ActionKind::Webhook,
        }
    }

    /// The action in canonical form (trimmed text), or why it cannot run.
    pub fn normalized(self) -> Result<Self, AutomationError> {
        match self {
            Self::ExportFile { format, folder } => {
                let folder = folder.trim().to_owned();
                if folder.is_empty() || !Path::new(&folder).is_absolute() {
                    return Err(AutomationError::invalid(
                        "export folder",
                        "must be a full path",
                    ));
                }
                Ok(Self::ExportFile { format, folder })
            }
            Self::Notify { message } => {
                let message = message.trim().to_owned();
                if message.chars().count() > MAX_MESSAGE_CHARS {
                    return Err(AutomationError::invalid("message", "is too long"));
                }
                Ok(Self::Notify { message })
            }
            Self::RunShell { command, fields } => {
                let command = command.trim().to_owned();
                let length = command.chars().count();
                if length == 0 || length > MAX_COMMAND_CHARS {
                    return Err(AutomationError::invalid(
                        "command",
                        "must be 1 to 4000 characters",
                    ));
                }
                Ok(Self::RunShell { command, fields })
            }
            action => Ok(action),
        }
    }
}

/// Every kind of step the builder lists. Kinds without an [`Action`]
/// variant are planned provider steps, shown as "WIP" (D12).
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    specta::Type,
    sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ActionKind {
    /// [`Action::GenerateInsight`].
    GenerateInsight,
    /// [`Action::AddTag`].
    AddTag,
    /// [`Action::AddToFolder`].
    AddToFolder,
    /// [`Action::ExportFile`].
    ExportFile,
    /// [`Action::Notify`].
    Notify,
    /// [`Action::RunShell`].
    RunShell,
    /// [`Action::Webhook`].
    Webhook,
    /// Notes to a Slack channel (INT-01).
    Slack,
    /// A Notion page or database row (INT-02).
    Notion,
    /// Tasks in Linear, Todoist and others (INT-03).
    Tasks,
    /// HubSpot, Salesforce or Attio (INT-04).
    Crm,
    /// A Google Doc or an Apple Note (INT-05).
    Document,
    /// An email draft or send through Gmail or Outlook (INT-07).
    Email,
}

impl ActionKind {
    /// The stored name (`automation_run_steps.kind`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GenerateInsight => "generate_insight",
            Self::AddTag => "add_tag",
            Self::AddToFolder => "add_to_folder",
            Self::ExportFile => "export_file",
            Self::Notify => "notify",
            Self::RunShell => "run_shell",
            Self::Webhook => "webhook",
            Self::Slack => "slack",
            Self::Notion => "notion",
            Self::Tasks => "tasks",
            Self::Crm => "crm",
            Self::Document => "document",
            Self::Email => "email",
        }
    }
}

/// What a step's JSON payload carries besides the meeting's ID, title and
/// times. Unset fields are left out of the payload entirely.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one independent switch per payload field, as the builder shows them"
)]
pub struct PayloadFields {
    /// The AI notes, as Markdown.
    pub notes: bool,
    /// The action items.
    pub action_items: bool,
    /// The transcript.
    pub transcript: bool,
    /// The insights' answers, including those earlier steps generated.
    pub insights: bool,
    /// Who took part.
    pub attendees: bool,
    /// The `deino://` link to the meeting.
    pub link: bool,
    /// Where the recording is on disk.
    pub audio_path: bool,
}

impl Default for PayloadFields {
    fn default() -> Self {
        Self {
            notes: true,
            action_items: true,
            transcript: false,
            insights: true,
            attendees: true,
            link: true,
            audio_path: false,
        }
    }
}
