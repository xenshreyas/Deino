//! The OS adapters behind `deino_core::export`'s ports.
//!
//! - [`TauriDialogs`]: the save and folder dialogs, shown as sheets on the
//!   main window (`tauri-plugin-dialog`);
//! - [`SystemClipboard`]: HTML plus a plain-text flavour through `arboard`,
//!   so rich text pastes formatted into Mail, Gmail, Docs and Slack and as
//!   text anywhere else;
//! - [`SystemOpener`]: hands a URL (`mailto:`) to the system, which opens
//!   the user's mail app.

use std::path::PathBuf;

use async_trait::async_trait;
use deino_core::export::{
    Clipboard, ClipboardContent, ExportError, FolderDialog, SaveDialog, SaveRequest, UrlOpener,
};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder, FilePath};
use tokio::sync::oneshot;

/// The save and folder dialogs.
pub struct TauriDialogs<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriDialogs<R> {
    /// Dialogs for `app`'s main window.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    /// A file dialog attached to the main window when it is open.
    fn builder(&self) -> FileDialogBuilder<R> {
        let dialog = self.app.dialog().file();
        match self.app.get_webview_window("main") {
            Some(window) => dialog.set_parent(&window),
            None => dialog,
        }
    }
}

/// Waits for a callback-style dialog's answer.
async fn answer(open: impl FnOnce(oneshot::Sender<Option<FilePath>>)) -> Option<PathBuf> {
    let (sender, receiver) = oneshot::channel();
    open(sender);
    receiver
        .await
        .ok()
        .flatten()
        .and_then(|path| path.into_path().ok())
}

#[async_trait]
impl<R: Runtime> SaveDialog for TauriDialogs<R> {
    async fn save_path(&self, request: SaveRequest) -> Option<PathBuf> {
        let dialog = self
            .builder()
            .set_file_name(request.file_name)
            .add_filter(request.kind, &[request.extension]);
        answer(|sender| {
            dialog.save_file(move |path| {
                // The command stopped waiting; nobody needs the answer.
                let _unheard = sender.send(path);
            });
        })
        .await
    }
}

#[async_trait]
impl<R: Runtime> FolderDialog for TauriDialogs<R> {
    async fn pick_folder(&self) -> Option<PathBuf> {
        let dialog = self.builder();
        answer(|sender| {
            dialog.pick_folder(move |path| {
                let _unheard = sender.send(path);
            });
        })
        .await
    }
}

/// The system clipboard.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClipboard;

#[async_trait]
impl Clipboard for SystemClipboard {
    async fn write(&self, content: ClipboardContent) -> Result<(), ExportError> {
        tokio::task::spawn_blocking(move || write_clipboard(&content))
            .await
            .map_err(|_| ExportError::Clipboard)?
    }
}

fn write_clipboard(content: &ClipboardContent) -> Result<(), ExportError> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| {
        log::warn!(target: "export", "the clipboard is not available: {error}");
        ExportError::Clipboard
    })?;
    let written = match &content.html {
        Some(html) => clipboard.set_html(html.as_str(), Some(content.text.as_str())),
        None => clipboard.set_text(content.text.as_str()),
    };
    written.map_err(|error| {
        log::warn!(target: "export", "could not write the clipboard: {error}");
        ExportError::Clipboard
    })
}

/// Opens URLs with the system's handler.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemOpener;

#[async_trait]
impl UrlOpener for SystemOpener {
    async fn open(&self, url: &str) -> Result<(), ExportError> {
        let status = opener_command(url).status().await.map_err(|error| {
            log::warn!(target: "export", "could not start the URL opener: {error}");
            ExportError::Mail
        })?;
        if status.success() {
            Ok(())
        } else {
            log::warn!(target: "export", "the URL opener exited with {status}");
            Err(ExportError::Mail)
        }
    }
}

/// The platform's "open this URL" command.
fn opener_command(url: &str) -> tokio::process::Command {
    #[cfg(target_os = "macos")]
    let mut command = tokio::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = tokio::process::Command::new("rundll32");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = tokio::process::Command::new("xdg-open");
    command.arg(url);
    command
}
