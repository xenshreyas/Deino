//! The dialogs behind the import ports: the open dialog filtered to the
//! formats imports accept, and the folder dialog for watched folders, shown
//! as sheets on the main window (`tauri-plugin-dialog`).

use std::path::PathBuf;

use async_trait::async_trait;
use deino_core::import::{AUDIO_EXTENSIONS, MediaPicker, VIDEO_EXTENSIONS};
use deino_core::watch::FolderPicker;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder, FilePath};
use tokio::sync::oneshot;

/// The open dialog's filter name.
const FILTER: &str = "Audio and video";

/// The import dialogs.
pub struct TauriImportDialogs<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriImportDialogs<R> {
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
impl<R: Runtime> MediaPicker for TauriImportDialogs<R> {
    async fn pick_media(&self) -> Option<PathBuf> {
        let extensions: Vec<&str> = AUDIO_EXTENSIONS
            .iter()
            .chain(&VIDEO_EXTENSIONS)
            .copied()
            .collect();
        let dialog = self.builder().add_filter(FILTER, &extensions);
        answer(|sender| {
            dialog.pick_file(move |path| {
                // The command stopped waiting; nobody needs the answer.
                let _unheard = sender.send(path);
            });
        })
        .await
    }
}

#[async_trait]
impl<R: Runtime> FolderPicker for TauriImportDialogs<R> {
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
