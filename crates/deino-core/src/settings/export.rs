//! The export section: auto-export after every meeting (EXP-02).
//!
//! Auto-export writes each meeting as a Markdown note into a folder the user
//! chose in a dialog (an Obsidian vault, say). The folder is a plain path:
//! Deino is not sandboxed. If it ever is, this field holds the path of a
//! security-scoped bookmark resolved by the app, and nothing else changes.

use std::path::Path;

use deino_export::ExportSections;
use deino_export::naming::NameTemplate;
use serde::{Deserialize, Serialize};

use super::SettingsError;

/// The default note name: `2026-10-08 Eng standup.md`.
pub const DEFAULT_FILE_NAME: &str = "{date} {title}.md";
/// The default folders inside the export folder: `2026/10`.
pub const DEFAULT_SUBFOLDER: &str = "{yyyy}/{mm}";

/// The export section of `Settings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent switches the user toggles one by one in Settings → Export"
)]
pub struct ExportSettings {
    /// Export every meeting when its pipeline finishes.
    pub auto_export: bool,
    /// The folder notes go into; chosen through the folder dialog only.
    pub folder: Option<String>,
    /// The note's file name, with `{date}`, `{time}`, `{title}`, `{yyyy}`,
    /// `{mm}` and `{dd}`.
    pub file_name: String,
    /// The folders inside `folder`, `/`-separated; empty for none.
    pub subfolder: String,
    /// What each note includes.
    pub sections: ExportSections,
    /// Rewrite a meeting's note when the meeting is processed again; when
    /// off, a note that exists is left as the user may have edited it.
    pub overwrite: bool,
    /// Write the transcript to its own note beside the meeting's.
    pub transcript_sidecar: bool,
    /// Obsidian mode: people as `[[wiki links]]` with their own pages, tags
    /// as `#tags`.
    pub obsidian: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            auto_export: false,
            folder: None,
            file_name: DEFAULT_FILE_NAME.to_owned(),
            subfolder: DEFAULT_SUBFOLDER.to_owned(),
            sections: ExportSections::default(),
            overwrite: true,
            transcript_sidecar: false,
            obsidian: false,
        }
    }
}

impl ExportSettings {
    /// Checks that the templates parse and the folder is absolute.
    pub fn validate(&self) -> Result<(), SettingsError> {
        if NameTemplate::parse(&self.file_name).is_err() {
            return Err(SettingsError::invalid(
                "export.fileName",
                "must be a name with {date}, {time}, {title}, {yyyy}, {mm} or {dd} placeholders",
            ));
        }
        if !self.subfolder.trim().is_empty() && NameTemplate::parse(&self.subfolder).is_err() {
            return Err(SettingsError::invalid(
                "export.subfolder",
                "must be folders with {date}, {time}, {title}, {yyyy}, {mm} or {dd} placeholders",
            ));
        }
        if self
            .folder
            .as_deref()
            .is_some_and(|folder| !Path::new(folder).is_absolute())
        {
            return Err(SettingsError::invalid(
                "export.folder",
                "must be an absolute path",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_export_is_off_until_the_user_turns_it_on() {
        let settings = ExportSettings::default();
        assert!(!settings.auto_export);
        assert_eq!(settings.folder, None);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn templates_with_unknown_placeholders_are_rejected() {
        let settings = ExportSettings {
            file_name: "{data} {title}".into(),
            ..ExportSettings::default()
        };
        assert!(matches!(
            settings.validate(),
            Err(SettingsError::InvalidValue {
                setting: "export.fileName",
                ..
            })
        ));
    }

    #[test]
    fn an_empty_subfolder_means_the_export_folder_itself() {
        let settings = ExportSettings {
            subfolder: String::new(),
            ..ExportSettings::default()
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn a_relative_folder_is_rejected() {
        let settings = ExportSettings {
            folder: Some("vault".into()),
            ..ExportSettings::default()
        };
        assert!(settings.validate().is_err());
    }
}
