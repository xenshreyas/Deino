//! [`SettingsPatch`]: an RFC 7396 JSON merge patch, the UI's way to change settings.
//!
//! The UI sends only what changed (`{ "ui": { "theme": "dark" } }`); `null`
//! resets a setting to its default. The patch is the one place untyped JSON
//! enters the core, and it never travels further: [`SettingsPatch::apply`]
//! merges it into the current settings and parses the result back into
//! [`Settings`], so a typo, a wrong type or a broken rule is rejected before
//! anything is saved.

use serde_json::{Map, Value};

use super::{Settings, SettingsError};

/// Settings the UI may clear but never set, as (section, field, path):
/// paths Deino writes to come only from dialogs Rust opens (EXP-02).
const DIALOG_ONLY: &[(&str, &str, &str)] = &[("export", "folder", "export.folder")];

/// A merge patch for [`Settings`]: a JSON object that does not touch
/// `version` and sets no [`DIALOG_ONLY`] setting.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsPatch(Map<String, Value>);

impl SettingsPatch {
    /// Accepts `value` as a patch if it is an object that leaves `version`
    /// alone and only clears dialog-only settings.
    pub fn parse(value: Value) -> Result<Self, SettingsError> {
        let Value::Object(fields) = value else {
            return Err(SettingsError::NotAnObject);
        };
        if fields.contains_key("version") {
            return Err(SettingsError::VersionIsReadOnly);
        }
        for (section, field, path) in DIALOG_ONLY {
            let set = fields
                .get(*section)
                .and_then(|section| section.get(*field))
                .is_some_and(|value| !value.is_null());
            if set {
                return Err(SettingsError::ChosenInDialog(path));
            }
        }
        Ok(Self(fields))
    }

    /// The settings `current` becomes with this patch applied, validated.
    pub fn apply(&self, current: &Settings) -> Result<Settings, SettingsError> {
        let mut document = serde_json::to_value(current).map_err(SettingsError::Encode)?;
        let patch = Value::Object(self.0.clone());
        json_patch::merge(&mut document, &patch);
        let next: Settings = serde_json::from_value(document).map_err(SettingsError::WrongShape)?;
        let canonical = serde_json::to_value(&next).map_err(SettingsError::Encode)?;
        if let Some(path) = unknown_path(&patch, &canonical, "") {
            return Err(SettingsError::UnknownSetting(path));
        }
        next.validate()?;
        Ok(next)
    }
}

/// The first field `patch` sets that `canonical` (the parsed result written
/// back out) does not have. Serde ignores unknown fields when parsing, so
/// this is where a misspelt setting is caught. `null` removes a field, so it
/// is never unknown.
fn unknown_path(patch: &Value, canonical: &Value, parent: &str) -> Option<String> {
    let (Value::Object(patch), Value::Object(canonical)) = (patch, canonical) else {
        return None;
    };
    patch
        .iter()
        .filter(|(_, value)| !value.is_null())
        .find_map(|(key, value)| {
            let setting = if parent.is_empty() {
                key.clone()
            } else {
                format!("{parent}.{key}")
            };
            match canonical.get(key) {
                None => Some(setting),
                Some(known) => unknown_path(value, known, &setting),
            }
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::settings::{LlmProvider, Theme};

    fn apply(patch: Value) -> Result<Settings, SettingsError> {
        SettingsPatch::parse(patch)?.apply(&Settings::default())
    }

    #[test]
    fn a_patch_changes_only_what_it_names() {
        let settings = apply(json!({ "ui": { "theme": "dark" } })).unwrap();

        assert_eq!(settings.ui.theme, Theme::Dark);
        assert!(settings.ui.show_confidence_indicator);
        assert_eq!(settings.ai, Settings::default().ai);
    }

    #[test]
    fn null_resets_a_setting_to_its_default() {
        let current =
            apply(json!({ "ai": { "ollamaEndpoint": "http://mac-mini:11434" } })).unwrap();

        let patch = SettingsPatch::parse(json!({ "ai": { "ollamaEndpoint": null } })).unwrap();
        let next = patch.apply(&current).unwrap();

        assert_eq!(next.ai.ollama_endpoint, None);
    }

    #[test]
    fn maps_merge_by_key() {
        let current = apply(json!({ "ai": { "providerModels": { "openai": "gpt-4o" } } })).unwrap();

        let patch =
            SettingsPatch::parse(json!({ "ai": { "providerModels": { "claude": "sonnet" } } }));
        let next = patch.unwrap().apply(&current).unwrap();

        assert_eq!(next.ai.provider_models.len(), 2);
        assert_eq!(next.ai.provider_models[&LlmProvider::Claude], "sonnet");
    }

    #[test]
    fn unknown_settings_are_rejected_with_their_path() {
        let error = apply(json!({ "ui": { "theem": "dark" } })).unwrap_err();

        assert_eq!(error.to_string(), "there is no setting named ui.theem");
    }

    #[test]
    fn wrong_types_and_broken_rules_are_rejected() {
        let wrong_type = apply(json!({ "recording": { "autoSave": "yes" } }));
        let bad_url = apply(json!({ "ai": { "ollamaEndpoint": "mac-mini" } }));

        assert!(matches!(wrong_type, Err(SettingsError::WrongShape(_))));
        assert!(matches!(
            bad_url,
            Err(SettingsError::InvalidValue {
                setting: "ai.ollamaEndpoint",
                ..
            })
        ));
    }

    #[test]
    fn only_objects_that_leave_the_version_alone_are_patches() {
        assert!(matches!(
            SettingsPatch::parse(json!([1])),
            Err(SettingsError::NotAnObject)
        ));
        assert!(matches!(
            SettingsPatch::parse(json!({ "version": 9 })),
            Err(SettingsError::VersionIsReadOnly)
        ));
    }

    #[test]
    fn the_export_folder_comes_only_from_the_dialog() {
        let set = SettingsPatch::parse(json!({ "export": { "folder": "/etc" } }));
        let cleared = apply(json!({ "export": { "folder": null, "autoExport": false } }));

        assert!(matches!(
            set,
            Err(SettingsError::ChosenInDialog("export.folder"))
        ));
        assert_eq!(cleared.unwrap().export.folder, None);
    }

    #[test]
    fn a_new_custom_endpoint_must_be_complete() {
        let half =
            apply(json!({ "ai": { "customOpenai": { "endpoint": "http://localhost:8000" } } }));

        assert!(matches!(
            half,
            Err(SettingsError::InvalidValue {
                setting: "ai.customOpenai.model",
                ..
            })
        ));
    }
}
