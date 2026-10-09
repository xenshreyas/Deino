//! [`ProfileInput`]: the profile as Settings → Profile sends it, normalised
//! into [`ProfileSettings`] before it is saved.

use deino_domain::Email;
use serde::Deserialize;

use super::ProfileError;
use crate::settings::ProfileSettings;

/// The profile form's values, as typed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileInput {
    /// The user's name.
    pub name: String,
    /// Email addresses in any case; blank entries are ignored.
    pub emails: Vec<String>,
    /// The user's role.
    pub role: String,
    /// The user's company.
    pub company: String,
    /// Focus areas; blank and repeated entries are ignored.
    pub focus_areas: Vec<String>,
    /// Writing-style notes.
    pub writing_style: String,
    /// Background and glossary.
    pub context: String,
}

impl ProfileInput {
    /// Trims every field, parses the addresses (lower case, each once, in
    /// order) and drops blank or repeated focus areas. Length limits are the
    /// settings' own rules (`ProfileSettings::validate`).
    pub fn normalize(self) -> Result<ProfileSettings, ProfileError> {
        Ok(ProfileSettings {
            name: self.name.trim().to_owned(),
            emails: parse_emails(&self.emails)?,
            role: self.role.trim().to_owned(),
            company: self.company.trim().to_owned(),
            focus_areas: unique_areas(&self.focus_areas),
            writing_style: self.writing_style.trim().to_owned(),
            context: self.context.trim().to_owned(),
        })
    }
}

/// The non-blank addresses, parsed, without repeats. The error names the
/// 1-based position of the first address that does not parse.
fn parse_emails(inputs: &[String]) -> Result<Vec<Email>, ProfileError> {
    let mut emails: Vec<Email> = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        if input.trim().is_empty() {
            continue;
        }
        let email = Email::parse(input).map_err(|_| ProfileError::InvalidEmail(index + 1))?;
        if !emails.contains(&email) {
            emails.push(email);
        }
    }
    Ok(emails)
}

/// Trimmed, non-blank focus areas, the first spelling of each kept.
fn unique_areas(inputs: &[String]) -> Vec<String> {
    let mut areas: Vec<String> = Vec::new();
    for area in inputs.iter().map(|area| area.trim()) {
        let repeated = areas.iter().any(|kept| kept.eq_ignore_ascii_case(area));
        if !area.is_empty() && !repeated {
            areas.push(area.to_owned());
        }
    }
    areas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_is_trimmed_lower_cased_and_deduplicated() {
        let input = ProfileInput {
            name: "  Shreyas ".into(),
            emails: vec![
                " Shreyas@Deino.dev".into(),
                String::new(),
                "shreyas@deino.dev".into(),
            ],
            focus_areas: vec!["Hiring".into(), " hiring ".into(), " ".into(), "Q4".into()],
            ..ProfileInput::default()
        };

        let profile = input.normalize().unwrap();

        assert_eq!(profile.name, "Shreyas");
        assert_eq!(profile.emails, [Email::parse("shreyas@deino.dev").unwrap()]);
        assert_eq!(profile.focus_areas, ["Hiring", "Q4"]);
    }

    #[test]
    fn a_malformed_address_is_named_by_its_position_not_its_text() {
        let input = ProfileInput {
            emails: vec!["a@deino.dev".into(), "not an address".into()],
            ..ProfileInput::default()
        };

        let error = input.normalize().unwrap_err();

        assert_eq!(error.to_string(), "email address 2 is not valid");
    }
}
