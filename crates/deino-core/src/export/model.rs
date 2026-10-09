//! The export feature's IPC types.

use deino_export::ExportFormat;
use serde::{Deserialize, Serialize};

/// What "Copy" puts on the clipboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CopyKind {
    /// The notes, action items and attendees as HTML with a plain-text
    /// fallback, so formatting survives in Gmail, Docs and Slack.
    RichText,
    /// The same as Markdown.
    Markdown,
    /// The transcript as plain text, with timestamps and speakers.
    Transcript,
}

/// How a preview is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PreviewKind {
    /// The file's text, shown as is (Markdown, TXT, JSON, subtitles).
    Text,
    /// An HTML rendering of a document format (PDF, DOCX).
    Html,
}

/// What the export dialog shows before saving.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreview {
    /// The format previewed.
    pub format: ExportFormat,
    /// How to show `content`.
    pub kind: PreviewKind,
    /// The file's text, or the document as an HTML fragment.
    pub content: String,
    /// The name the save dialog will suggest.
    pub file_name: String,
}

/// Where the account export is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum FullExportState {
    /// No export has run since launch.
    Idle,
    /// Writing the archive.
    Running,
    /// The archive is saved.
    Finished,
    /// The export stopped on an error.
    Failed,
    /// The user cancelled it.
    Cancelled,
}

/// The account export's progress.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FullExportStatus {
    /// Where it is.
    pub state: FullExportState,
    /// Meetings written so far.
    pub done: u32,
    /// Meetings to write.
    pub total: u32,
    /// The archive's name, once finished.
    pub file_name: Option<String>,
    /// Why it failed (user-safe).
    pub message: Option<String>,
}

impl FullExportStatus {
    /// Nothing has run.
    pub fn idle() -> Self {
        Self::ended(FullExportState::Idle, None)
    }

    /// `done` of `total` meetings written.
    pub fn running(done: u32, total: u32) -> Self {
        Self {
            state: FullExportState::Running,
            done,
            total,
            file_name: None,
            message: None,
        }
    }

    /// All `total` meetings are in the archive named `file_name`.
    pub fn finished(total: u32, file_name: Option<String>) -> Self {
        Self {
            state: FullExportState::Finished,
            done: total,
            total,
            file_name,
            message: None,
        }
    }

    /// Stopped in `state`, with an optional reason.
    pub fn ended(state: FullExportState, message: Option<String>) -> Self {
        Self {
            state,
            done: 0,
            total: 0,
            file_name: None,
            message,
        }
    }
}

/// A file the user saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedExport {
    /// The file's name (not its folder), for the confirmation.
    pub file_name: String,
}
