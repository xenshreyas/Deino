//! The settings event port: tells listeners that settings changed.
//!
//! The app implements it with the typed `SettingsChanged` IPC event, which
//! the UI's `useSettings()` writes straight into its cache. Settings are not
//! in SQLite, so `DbChanged` cannot do this job.

use super::Settings;

/// Receives every committed settings change.
pub trait SettingsEvents: Send + Sync {
    /// `settings` were saved and are now current.
    fn changed(&self, settings: &Settings);
}
