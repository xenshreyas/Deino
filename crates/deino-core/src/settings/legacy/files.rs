//! [`LegacyFileImport`]: moves the legacy JSON stores into [`Settings`].
//!
//! The stores sit in the app data dir: four tauri-plugin-store files and the
//! notification manager's file. None of them ever held a key. Each file that
//! parses is applied, then deleted once the new settings are saved; a file
//! that does not parse is left in place and logged. Two obsolete stores
//! (`store.json`, the retired server address, and `analytics.json`) are
//! deleted unread.

use std::io;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use super::apply_if_valid;
use super::formats::{NotificationsFile, OnboardingFile, PreferencesFile, RecordingFile};
use crate::settings::{Settings, SettingsError, SettingsService};

/// Store files that hold nothing worth keeping.
const OBSOLETE_FILES: [&str; 2] = ["store.json", "analytics.json"];

/// The legacy stores found on disk, parsed.
#[derive(Debug, Default)]
struct Found {
    recording: Option<RecordingFile>,
    preferences: Option<PreferencesFile>,
    onboarding: Option<OnboardingFile>,
    notifications: Option<NotificationsFile>,
    /// Every file to delete once the settings are saved.
    imported: Vec<PathBuf>,
}

/// Imports the legacy JSON stores from one directory.
#[derive(Clone, Debug)]
pub struct LegacyFileImport {
    directory: PathBuf,
}

impl LegacyFileImport {
    /// An import from `directory`, the app data dir.
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    /// Applies every legacy store found, saves, then deletes the stores.
    pub async fn run(&self, settings: &SettingsService) -> Result<(), SettingsError> {
        let found = self.find();
        if found.imported.is_empty() {
            return Ok(());
        }
        let files = found.imported.len();
        let imported = found.imported.clone();
        settings.update(|current| found.apply(current)).await?;
        for path in imported {
            remove(&path);
        }
        log::info!(target: "settings", "moved {files} legacy settings files");
        Ok(())
    }

    fn find(&self) -> Found {
        let mut found = Found {
            recording: self.read("recording_preferences.json"),
            preferences: self.read("preferences.json"),
            onboarding: self.read("onboarding-status.json"),
            notifications: self.read("notifications.json"),
            imported: Vec::new(),
        };
        let names = [
            ("recording_preferences.json", found.recording.is_some()),
            ("preferences.json", found.preferences.is_some()),
            ("onboarding-status.json", found.onboarding.is_some()),
            ("notifications.json", found.notifications.is_some()),
        ];
        let parsed = names.iter().filter(|(_, ok)| *ok).map(|(name, _)| *name);
        let obsolete = OBSOLETE_FILES
            .into_iter()
            .filter(|name| self.path(name).exists());
        found.imported = parsed.chain(obsolete).map(|name| self.path(name)).collect();
        found
    }

    /// The parsed store `name`; `None` when it is missing or unreadable.
    fn read<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        let bytes = match std::fs::read(self.path(name)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
            Err(error) => {
                log::warn!(target: "settings", "could not read legacy {name}: {error}");
                return None;
            }
        };
        serde_json::from_slice(&bytes)
            .inspect_err(|error| {
                log::warn!(target: "settings", "left the unreadable legacy {name} in place: {error}");
            })
            .ok()
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }
}

impl Found {
    fn apply(self, settings: &mut Settings) {
        if let Some(file) = self.recording {
            apply_if_valid(settings, "recording", |s| file.apply(s));
        }
        if let Some(file) = self.preferences {
            apply_if_valid(settings, "recording notice", |s| file.apply(s));
        }
        if let Some(file) = self.onboarding {
            apply_if_valid(settings, "onboarding", |s| file.apply(s));
        }
        if let Some(file) = self.notifications {
            apply_if_valid(settings, "notification", |s| file.apply(s));
        }
    }
}

/// Deletes a moved store; a failure only means it is imported again next launch.
fn remove(path: &Path) {
    if let Err(error) = std::fs::remove_file(path) {
        log::warn!(target: "settings", "could not delete legacy {}: {error}", path.display());
    }
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
