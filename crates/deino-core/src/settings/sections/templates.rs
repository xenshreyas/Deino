//! The templates section (NOT-04): which note template meetings get by default.

use deino_domain::TemplateId;
use serde::{Deserialize, Serialize};

/// The templates section of `Settings`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TemplateSettings {
    /// The template a meeting gets when the user has not chosen one and no
    /// template's auto rules match. `None` is "Auto": the LLM classifier may
    /// suggest one, and otherwise the General template is used. A default
    /// that names a deleted template falls back the same way.
    pub default_template: Option<TemplateId>,
}

crate::settings::register_section! {
    /// Note templates (NOT-04).
    templates: TemplateSettings,
    order: 120,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_is_the_default_and_serializes_as_null() {
        let json = serde_json::to_value(TemplateSettings::default()).unwrap();

        assert_eq!(json, serde_json::json!({ "defaultTemplate": null }));
    }
}
