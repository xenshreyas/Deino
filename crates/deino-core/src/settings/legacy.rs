//! One-time import of the settings stores that predate FND-06.
//!
//! Before FND-06, settings were scattered over SQLite (`settings`,
//! `transcript_settings`, with API keys in plaintext), tauri-plugin-store
//! JSON files and the webview's localStorage. Two importers move them:
//!
//! - [`LegacySqliteImport`], a `deino_db::DataMigration`, copies the SQLite
//!   rows into [`Settings`] and the keys into the Keychain just before the
//!   migration that drops both tables;
//! - [`LegacyFileImport`] copies the JSON stores at startup, then deletes
//!   them.
//!
//! The UI moves its own localStorage keys (`features/settings` in the
//! frontend). Every importer is idempotent: it runs on each launch and finds
//! nothing once the old store is gone. A legacy value that breaks one of
//! today's rules (an endpoint without a scheme, say) is skipped with a
//! warning; it never stops the app from starting.

mod files;
mod formats;
mod sqlite;

pub use files::LegacyFileImport;
pub use sqlite::{DROP_LEGACY_SETTINGS_VERSION, LegacySqliteImport};

use super::Settings;

/// Applies `change` to `settings` unless the result breaks a rule, in which
/// case `settings` is left alone and the skipped `part` is logged.
fn apply_if_valid(settings: &mut Settings, part: &str, change: impl FnOnce(&mut Settings)) {
    let mut next = settings.clone();
    change(&mut next);
    match next.validate() {
        Ok(()) => *settings = next,
        Err(error) => {
            log::warn!(target: "settings", "skipped legacy {part} settings: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_that_breaks_a_rule_is_skipped_whole() {
        let mut settings = Settings::default();

        apply_if_valid(&mut settings, "ui", |s| {
            s.ui.show_confidence_indicator = false;
            s.ui.transcript_pane_ratio = 9.0;
        });

        assert_eq!(settings, Settings::default());
    }
}
