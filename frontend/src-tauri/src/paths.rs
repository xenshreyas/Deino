//! [`AppPaths`]: every directory the app uses, resolved once at startup.
//!
//! Paths come from Tauri's resolver, so they follow the bundle identifier
//! (a smoke build with another identifier gets its own folders). The
//! composition root hands them to whatever needs them; nothing else calls
//! the resolver or `dirs::` for app folders.

use std::path::PathBuf;

use deino_core::settings::JsonFileSettingsStore;
use tauri::{AppHandle, Manager, Runtime};

/// The app's directories.
#[derive(Clone, Debug)]
pub(crate) struct AppPaths {
    /// Configuration: `settings.json`.
    config: PathBuf,
    /// Data: the database, models and the legacy stores.
    data: PathBuf,
}

impl AppPaths {
    /// Resolves every directory for `app`.
    pub(crate) fn resolve<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Self> {
        let resolver = app.path();
        Ok(Self {
            config: resolver.app_config_dir()?,
            data: resolver.app_data_dir()?,
        })
    }

    /// The typed settings file.
    pub(crate) fn settings_file(&self) -> PathBuf {
        self.config.join(JsonFileSettingsStore::FILE_NAME)
    }

    /// The app data dir, where the legacy settings stores were written.
    pub(crate) fn data(&self) -> &PathBuf {
        &self.data
    }

    /// Downloaded transcription models (`<data>/models`).
    pub(crate) fn models(&self) -> PathBuf {
        self.data.join("models")
    }

    /// One folder per meeting (`<data>/meetings/<meeting id>`), holding the
    /// note images (NOT-01). Inside the asset protocol's `$APPDATA` scope.
    pub(crate) fn meetings(&self) -> PathBuf {
        self.data.join("meetings")
    }

    /// The pre-NOT-04 custom templates folder (`<data>/templates`), read
    /// once at startup to import its JSON files into the database.
    pub(crate) fn legacy_templates(&self) -> PathBuf {
        self.data.join("templates")
    }
}
