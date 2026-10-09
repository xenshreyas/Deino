//! What exporting needs from the operating system, as ports the app
//! implements: the save and folder dialogs, the clipboard and the URL opener.
//!
//! The dialogs are the only source of paths: the webview never names a file
//! to write, so a compromised page cannot write anywhere.

use std::path::PathBuf;

use async_trait::async_trait;

use super::ExportError;

/// What the save dialog proposes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRequest {
    /// The suggested file name, with its extension.
    pub file_name: String,
    /// The file type's name in the dialog's filter ("PDF document").
    pub kind: &'static str,
    /// The extension the filter allows, without the dot.
    pub extension: &'static str,
}

/// Asks the user where to save a file.
#[async_trait]
pub trait SaveDialog: Send + Sync {
    /// The path the user chose, or `None` when they cancelled.
    async fn save_path(&self, request: SaveRequest) -> Option<PathBuf>;
}

/// Asks the user for a folder.
#[async_trait]
pub trait FolderDialog: Send + Sync {
    /// The folder the user chose, or `None` when they cancelled.
    async fn pick_folder(&self) -> Option<PathBuf>;
}

/// What goes on the clipboard: plain text, and the same as HTML when the
/// copy is rich.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardContent {
    /// The plain-text flavour, always present.
    pub text: String,
    /// The HTML flavour, for rich text.
    pub html: Option<String>,
}

/// The system clipboard.
#[async_trait]
pub trait Clipboard: Send + Sync {
    /// Replaces the clipboard's contents.
    async fn write(&self, content: ClipboardContent) -> Result<(), ExportError>;
}

/// Opens URLs in the app that handles them (`mailto:` in Mail).
#[async_trait]
pub trait UrlOpener: Send + Sync {
    /// Opens `url`.
    async fn open(&self, url: &str) -> Result<(), ExportError>;
}
