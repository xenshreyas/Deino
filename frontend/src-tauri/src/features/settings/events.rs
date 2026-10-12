//! The `settings` plugin's typed event and the adapter that emits it.
//!
//! The core announces each saved change through its `SettingsEvents` port;
//! [`TauriSettingsEvents`] emits it as `plugin:settings:settings-changed`
//! and hands the app lock its new settings.

use std::sync::Arc;

use deino_core::lock::LockService;
use deino_core::settings::{Settings, SettingsEvents};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

/// The settings were saved; `settings` is the new, complete document.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
pub struct SettingsChanged {
    /// Every setting, as now saved.
    pub settings: Settings,
}

/// Emits settings changes to the UI.
pub struct TauriSettingsEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriSettingsEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> SettingsEvents for TauriSettingsEvents<R> {
    fn changed(&self, settings: &Settings) {
        // The app lock follows its settings at once (PRV-02).
        if let Some(lock) = self.app.try_state::<Arc<LockService>>() {
            lock.apply(crate::app_lock::supported(settings.privacy.app_lock));
        }
        let event = SettingsChanged {
            settings: settings.clone(),
        };
        // The change is saved either way; a window that misses it refetches
        // the next time it mounts the settings query.
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "settings", "could not emit settings-changed: {error}");
        }
    }
}
