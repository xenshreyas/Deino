//! The profile section (NOT-05): who the user is, for every AI prompt.
//!
//! The profile becomes a short system-context snippet
//! (`profile::render_profile_context`), its emails identify the "Me" person
//! and tell internal meetings from external ones by domain
//! (`profile::InternalDomains`). Settings → Profile edits it through
//! `profile::ProfileService`, which normalises the input before it lands here.

use deino_domain::Email;
use serde::{Deserialize, Serialize};

use super::SettingsError;

/// The longest name, role or company, in characters.
pub const MAX_PROFILE_LINE_CHARS: usize = 120;
/// The longest writing-style note, in characters.
pub const MAX_WRITING_STYLE_CHARS: usize = 1_000;
/// The longest glossary/context note, in characters. It goes into every
/// prompt, so it stays short enough not to crowd out the transcript.
pub const MAX_CONTEXT_CHARS: usize = 4_000;
/// The most email addresses a profile can list.
pub const MAX_PROFILE_EMAILS: usize = 10;
/// The most focus areas a profile can list.
pub const MAX_FOCUS_AREAS: usize = 12;
/// The longest focus area, in characters.
pub const MAX_FOCUS_AREA_CHARS: usize = 60;

/// The profile section of `Settings`. Every field may be empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileSettings {
    /// The user's name, as attendees know them.
    pub name: String,
    /// The user's addresses, work first. The first is the "Me" person's
    /// email; every address's domain (except personal mail providers) counts
    /// as internal.
    pub emails: Vec<Email>,
    /// The user's role, for example "CTO" or "Account executive".
    pub role: String,
    /// The user's company.
    pub company: String,
    /// What the user cares about most, so notes emphasise it.
    pub focus_areas: Vec<String>,
    /// How the user likes notes written ("short bullets, British spelling").
    pub writing_style: String,
    /// Background and a glossary of names, products and acronyms.
    pub context: String,
}

impl ProfileSettings {
    /// Checks the limits that keep the profile prompt-sized.
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        for (setting, value) in [
            ("profile.name", &self.name),
            ("profile.role", &self.role),
            ("profile.company", &self.company),
        ] {
            require_at_most(setting, value, MAX_PROFILE_LINE_CHARS)?;
        }
        require_at_most(
            "profile.writingStyle",
            &self.writing_style,
            MAX_WRITING_STYLE_CHARS,
        )?;
        require_at_most("profile.context", &self.context, MAX_CONTEXT_CHARS)?;
        self.validate_lists()
    }

    fn validate_lists(&self) -> Result<(), SettingsError> {
        if self.emails.len() > MAX_PROFILE_EMAILS {
            return Err(SettingsError::invalid(
                "profile.emails",
                "has too many addresses",
            ));
        }
        let mut unique = self.emails.clone();
        unique.sort();
        unique.dedup();
        if unique.len() != self.emails.len() {
            return Err(SettingsError::invalid(
                "profile.emails",
                "lists an address twice",
            ));
        }
        if self.focus_areas.len() > MAX_FOCUS_AREAS {
            return Err(SettingsError::invalid(
                "profile.focusAreas",
                "has too many entries",
            ));
        }
        self.focus_areas.iter().try_for_each(|area| {
            if area.trim().is_empty() {
                return Err(SettingsError::invalid(
                    "profile.focusAreas",
                    "has a blank entry",
                ));
            }
            require_at_most("profile.focusAreas", area, MAX_FOCUS_AREA_CHARS)
        })
    }

    /// The address the "Me" person carries: the first one listed.
    pub fn primary_email(&self) -> Option<&Email> {
        self.emails.first()
    }
}

/// Fails when `value` is longer than `limit` characters.
fn require_at_most(setting: &'static str, value: &str, limit: usize) -> Result<(), SettingsError> {
    if value.chars().count() > limit {
        return Err(SettingsError::invalid(setting, "is too long"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn email(address: &str) -> Email {
        Email::parse(address).unwrap()
    }

    #[test]
    fn the_empty_profile_is_valid() {
        assert!(ProfileSettings::default().validate().is_ok());
    }

    #[test]
    fn a_full_profile_round_trips_with_camel_case_keys() {
        let profile = ProfileSettings {
            name: "Shreyas".into(),
            emails: vec![email("shreyas@deino.dev")],
            focus_areas: vec!["Reliability".into()],
            writing_style: "Short bullets".into(),
            ..ProfileSettings::default()
        };

        let json = serde_json::to_value(&profile).unwrap();

        assert_eq!(json["focusAreas"], serde_json::json!(["Reliability"]));
        assert_eq!(json["writingStyle"], "Short bullets");
        assert_eq!(
            serde_json::from_value::<ProfileSettings>(json).unwrap(),
            profile
        );
        assert_eq!(profile.primary_email(), Some(&email("shreyas@deino.dev")));
    }

    #[test]
    fn limits_and_duplicates_are_rejected_with_the_setting_path() {
        let long = ProfileSettings {
            context: "x".repeat(MAX_CONTEXT_CHARS + 1),
            ..ProfileSettings::default()
        };
        let twice = ProfileSettings {
            emails: vec![email("a@deino.dev"), email("a@deino.dev")],
            ..ProfileSettings::default()
        };
        let blank_area = ProfileSettings {
            focus_areas: vec!["  ".into()],
            ..ProfileSettings::default()
        };

        assert_eq!(
            long.validate().unwrap_err().to_string(),
            "profile.context is too long"
        );
        assert_eq!(
            twice.validate().unwrap_err().to_string(),
            "profile.emails lists an address twice"
        );
        assert!(blank_area.validate().is_err());
    }
}
