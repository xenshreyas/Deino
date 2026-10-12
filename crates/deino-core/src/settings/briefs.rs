//! The briefs section: whether Deino prepares pre-meeting briefs, and for
//! which meetings (NOT-08).

use serde::{Deserialize, Serialize};

/// Which upcoming meetings get a brief.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum BriefAudience {
    /// Meetings with someone outside the user's organisation (Granola's
    /// rule).
    #[default]
    External,
    /// Every meeting with at least two attendees, internal ones included.
    Groups,
}

/// The briefs section of `Settings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct BriefSettings {
    /// Prepare briefs at 06:00 and an hour before each meeting. On by
    /// default; briefs read only the user's own data (no web, no email).
    pub enabled: bool,
    /// Which meetings get one.
    pub audience: BriefAudience,
}

impl Default for BriefSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            audience: BriefAudience::External,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn briefs_are_on_for_external_meetings_unless_the_file_says_otherwise() {
        let missing: BriefSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(missing, BriefSettings::default());
        assert!(missing.enabled);
        assert_eq!(missing.audience, BriefAudience::External);

        let groups: BriefSettings =
            serde_json::from_str(r#"{ "enabled": false, "audience": "groups" }"#).unwrap();
        assert!(!groups.enabled);
        assert_eq!(groups.audience, BriefAudience::Groups);
    }
}
