//! The organize section: how Deino files and labels meetings by itself
//! (ORG-05, ORG-06).

use serde::{Deserialize, Serialize};

/// The organize section of `Settings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct OrganizeSettings {
    /// Let the AI apply existing tags to each meeting after processing
    /// (the `auto_tag` pipeline stage). It never creates tags, and it skips
    /// meetings the user tagged by hand. On by default, as in Circleback.
    pub auto_tag: bool,
}

impl Default for OrganizeSettings {
    fn default() -> Self {
        Self { auto_tag: true }
    }
}

crate::settings::register_section! {
    /// Auto-tagging and auto-filing (ORG-05, ORG-06).
    organize: OrganizeSettings,
    order: 130,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_tagging_is_on_unless_the_file_turns_it_off() {
        assert!(OrganizeSettings::default().auto_tag);
        let off: OrganizeSettings = serde_json::from_str(r#"{ "autoTag": false }"#).unwrap();
        assert!(!off.auto_tag);
        let missing: OrganizeSettings = serde_json::from_str("{}").unwrap();
        assert!(missing.auto_tag);
    }
}
