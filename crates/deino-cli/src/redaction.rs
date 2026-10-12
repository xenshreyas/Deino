//! Redaction for the CLI and the MCP server (NOT-09): when the app's
//! Settings → Privacy → "Hide sensitive information" is on, what `deino`
//! prints, exports and hands to Claude has its secrets `[redacted]`, by
//! patterns alone (the CLI never calls a model; ADR 0038).
//!
//! The setting is read from the app's `settings.json`: beside the database
//! (on macOS the app keeps its config and data in one folder), else in the
//! platform's config folder. A missing or unreadable file means the
//! default, off, as it does for the app.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use deino_core::redaction::{NoRedaction, Redactor, TextRedactor};
use deino_core::settings::{JsonFileSettingsStore, Settings};

use crate::db_path::{APP_IDENTIFIER, app_database};

/// The redactor the app's setting asks for, for the database at `database`.
pub fn redactor_for(database: &Path) -> Arc<dyn TextRedactor> {
    if settings_files(database)
        .iter()
        .find_map(|path| enabled_in(path))
        == Some(true)
    {
        Arc::new(Redactor::regex_only())
    } else {
        Arc::new(NoRedaction)
    }
}

/// Where the app's settings may be, most specific first: beside the
/// database, and for the app's own database also the config folder (where
/// Linux keeps it apart from the data). Another `--db` never reads the
/// user's own settings.
fn settings_files(database: &Path) -> Vec<PathBuf> {
    let beside = database
        .parent()
        .map(|folder| folder.join(JsonFileSettingsStore::FILE_NAME));
    let config = dirs::config_dir()
        .filter(|_| app_database().as_deref() == Some(database))
        .map(|config| {
            config
                .join(APP_IDENTIFIER)
                .join(JsonFileSettingsStore::FILE_NAME)
        });
    beside.into_iter().chain(config).collect()
}

/// The setting in the file at `path`; `None` when there is no readable file.
fn enabled_in(path: &Path) -> Option<bool> {
    let bytes = std::fs::read(path).ok()?;
    let settings = serde_json::from_slice::<Settings>(&bytes).ok()?.upgraded();
    Some(settings.privacy.redact_sensitive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setting_is_read_from_beside_the_database() {
        let folder = tempfile::tempdir().unwrap();
        let settings = folder.path().join(JsonFileSettingsStore::FILE_NAME);
        let database = folder.path().join("deino.sqlite");

        std::fs::write(&settings, r#"{"privacy":{"redactSensitive":true}}"#).unwrap();
        assert_eq!(enabled_in(&settings), Some(true));
        assert_eq!(settings_files(&database).first(), Some(&settings));

        std::fs::write(&settings, r#"{"privacy":{}}"#).unwrap();
        assert_eq!(enabled_in(&settings), Some(false));

        std::fs::write(&settings, "not json").unwrap();
        assert_eq!(enabled_in(&settings), None);
    }
}
