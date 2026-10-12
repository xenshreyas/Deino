//! `deino_core::links`' ports over the app: the recorder (CAP-10's
//! `RecordingService::start`/`stop`, the menu bar's path), the app lock
//! and the main window; and [`follow`], which runs an intent off the
//! main thread and reports a refusal to the UI.

use std::future::Future;
use std::sync::Arc;

use async_trait::async_trait;
use deino_core::links::{
    Intent, LinkError, LinkService, LockProbe, Recorder, RecorderError, Shell,
};
use deino_core::lock::LockService;
use deino_domain::{ErrorClass, ErrorCode};
use deino_recording::{RecordingPhase, RecordingService, StartRequest};
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use super::events::{LinkRejected, NavigationWaiting};

/// The live recorder, from the app's state.
pub(crate) struct AppRecorder<R: Runtime>(pub(crate) AppHandle<R>);

impl<R: Runtime> AppRecorder<R> {
    fn service(&self) -> Result<Arc<RecordingService>, RecorderError> {
        self.0
            .try_state::<Arc<RecordingService>>()
            .map(|state| Arc::clone(&state))
            .ok_or_else(|| RecorderError {
                code: ErrorCode::Unavailable,
                message: "the recorder is not ready yet".to_owned(),
            })
    }
}

fn recorder_error(error: &(impl ErrorClass + std::fmt::Display)) -> RecorderError {
    RecorderError {
        code: error.code(),
        message: error.to_string(),
    }
}

#[async_trait]
impl<R: Runtime> Recorder for AppRecorder<R> {
    fn is_recording(&self) -> bool {
        self.service()
            .is_ok_and(|recorder| recorder.state().phase != RecordingPhase::Idle)
    }

    async fn start(&self) -> Result<(), RecorderError> {
        let recorder = self.service()?;
        recorder
            .start(StartRequest::default())
            .await
            .map(drop)
            .map_err(|error| recorder_error(&error))
    }

    async fn stop(&self) -> Result<(), RecorderError> {
        let recorder = self.service()?;
        recorder
            .stop()
            .await
            .map(drop)
            .map_err(|error| recorder_error(&error))
    }
}

/// The app lock (PRV-02); unlocked when it is not wired (no lock yet).
pub(crate) struct AppLock<R: Runtime>(pub(crate) AppHandle<R>);

impl<R: Runtime> LockProbe for AppLock<R> {
    fn is_locked(&self) -> bool {
        self.0
            .try_state::<Arc<LockService>>()
            .is_some_and(|lock| lock.is_locked())
    }
}

/// The main window.
pub(crate) struct AppShell<R: Runtime>(pub(crate) AppHandle<R>);

impl<R: Runtime> Shell for AppShell<R> {
    fn focus(&self) {
        crate::tray::focus_main_window(&self.0);
    }

    fn navigation_waiting(&self) {
        if let Err(error) = (NavigationWaiting {}).emit(&self.0) {
            log::warn!(target: "links", "could not tell the window: {error}");
        }
    }
}

/// Opens each of `urls` in turn, off the main thread.
pub(crate) fn open_urls<R: Runtime>(app: &AppHandle<R>, urls: Vec<String>) {
    for url in urls {
        follow(app, move |links| async move { links.open(&url).await });
    }
}

/// Runs `intent` (a hotkey's), off the main thread.
pub(crate) fn perform<R: Runtime>(app: &AppHandle<R>, intent: Intent) {
    follow(app, move |links| async move { links.perform(intent).await });
}

/// Runs `act` on the link service, and tells the UI when it fails.
fn follow<R, F, Fut>(app: &AppHandle<R>, act: F)
where
    R: Runtime,
    F: FnOnce(Arc<LinkService>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), LinkError>> + Send,
{
    let Some(links) = app.try_state::<Arc<LinkService>>().map(|s| Arc::clone(&s)) else {
        return log::warn!(target: "links", "a link arrived before the link service");
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = act(links).await {
            log::info!(target: "links", "link not followed: {error}");
            let message = error.to_string();
            if let Err(error) = (LinkRejected { message }).emit(&app) {
                log::warn!(target: "links", "could not report the link: {error}");
            }
        }
    });
}
