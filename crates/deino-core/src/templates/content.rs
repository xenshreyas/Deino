//! [`TemplateContent`]: what a template says, validated before it is stored.
//!
//! A template is a prompt document (Granola's model, `docs/research/granola.md`
//! §3.4): a name and purpose, context about the meeting, an ordered list of
//! sections with per-section instructions and a format, and the auto rules
//! that pick it.

use deino_domain::SectionFormat;
use serde::{Deserialize, Serialize};

use super::{AutoRule, MAX_AUTO_RULES, TemplateError};

/// The longest template name, in characters.
pub const MAX_NAME_CHARS: usize = 80;
/// The longest description, in characters.
pub const MAX_DESCRIPTION_CHARS: usize = 300;
/// The longest meeting context, in characters.
pub const MAX_MEETING_CONTEXT_CHARS: usize = 2_000;
/// The most sections a template can have.
pub const MAX_SECTIONS: usize = 20;
/// The longest section title, in characters.
pub const MAX_SECTION_TITLE_CHARS: usize = 80;
/// The longest section instructions, in characters.
pub const MAX_INSTRUCTIONS_CHARS: usize = 1_000;

/// One section of the notes a template produces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSection {
    /// The heading, for example "Action items".
    pub title: String,
    /// What the LLM should put in it; may be empty for an obvious title.
    #[serde(default)]
    pub instructions: String,
    /// How it is rendered.
    pub format: SectionFormat,
}

/// Everything about a template the user can edit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TemplateContent {
    /// The name in pickers, for example "Sales discovery".
    pub name: String,
    /// One line on what it is for; the picker shows it and the LLM
    /// classifier chooses by it.
    #[serde(default)]
    pub description: String,
    /// Who the meeting is with and what the notes are for, given to the LLM
    /// with every section.
    #[serde(default)]
    pub meeting_context: String,
    /// The sections, in order.
    pub sections: Vec<TemplateSection>,
    /// When to choose this template automatically.
    #[serde(default)]
    pub auto_rules: Vec<AutoRule>,
}

impl TemplateContent {
    /// The content with every field trimmed and every rule checked, or the
    /// first broken rule. Section titles must be unique, ignoring case, so
    /// the scaffold and the notes never have two identical headings.
    pub fn validated(self) -> Result<Self, TemplateError> {
        let name = bounded("name", &self.name, MAX_NAME_CHARS)?;
        if name.is_empty() {
            return Err(TemplateError::invalid("name", "cannot be empty"));
        }
        Ok(Self {
            name,
            description: bounded("description", &self.description, MAX_DESCRIPTION_CHARS)?,
            meeting_context: bounded(
                "meeting context",
                &self.meeting_context,
                MAX_MEETING_CONTEXT_CHARS,
            )?,
            sections: validated_sections(self.sections)?,
            auto_rules: validated_rules(self.auto_rules)?,
        })
    }
}

/// Trims `value` and fails when it is longer than `limit` characters.
fn bounded(field: &'static str, value: &str, limit: usize) -> Result<String, TemplateError> {
    let value = value.trim();
    if value.chars().count() > limit {
        return Err(TemplateError::invalid(field, "is too long"));
    }
    Ok(value.to_owned())
}

fn validated_sections(
    sections: Vec<TemplateSection>,
) -> Result<Vec<TemplateSection>, TemplateError> {
    if sections.is_empty() {
        return Err(TemplateError::invalid(
            "sections",
            "need at least one section",
        ));
    }
    if sections.len() > MAX_SECTIONS {
        return Err(TemplateError::invalid("sections", "are too many"));
    }
    let mut checked: Vec<TemplateSection> = Vec::with_capacity(sections.len());
    for (index, section) in sections.into_iter().enumerate() {
        let section = validated_section(&section, index + 1)?;
        let repeated = checked
            .iter()
            .any(|earlier| earlier.title.to_lowercase() == section.title.to_lowercase());
        if repeated {
            return Err(TemplateError::InvalidSection {
                position: index + 1,
                rule: "repeats an earlier title",
            });
        }
        checked.push(section);
    }
    Ok(checked)
}

fn validated_section(
    section: &TemplateSection,
    position: usize,
) -> Result<TemplateSection, TemplateError> {
    let invalid = |rule| TemplateError::InvalidSection { position, rule };
    let title = section.title.trim().to_owned();
    if title.is_empty() {
        return Err(invalid("needs a title"));
    }
    if title.chars().count() > MAX_SECTION_TITLE_CHARS {
        return Err(invalid("has a title that is too long"));
    }
    let instructions = section.instructions.trim().to_owned();
    if instructions.chars().count() > MAX_INSTRUCTIONS_CHARS {
        return Err(invalid("has instructions that are too long"));
    }
    Ok(TemplateSection {
        title,
        instructions,
        format: section.format,
    })
}

fn validated_rules(rules: Vec<AutoRule>) -> Result<Vec<AutoRule>, TemplateError> {
    if rules.len() > MAX_AUTO_RULES {
        return Err(TemplateError::invalid("auto rules", "are too many"));
    }
    rules
        .into_iter()
        .enumerate()
        .map(|(index, rule)| rule.validated(index + 1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(title: &str) -> TemplateSection {
        TemplateSection {
            title: title.into(),
            instructions: String::new(),
            format: SectionFormat::Bullets,
        }
    }

    fn content(sections: Vec<TemplateSection>) -> TemplateContent {
        TemplateContent {
            name: " Board prep ".into(),
            description: " For board decks ".into(),
            meeting_context: String::new(),
            sections,
            auto_rules: Vec::new(),
        }
    }

    #[test]
    fn content_is_trimmed() {
        let checked = content(vec![section(" Overview ")]).validated().unwrap();

        assert_eq!(checked.name, "Board prep");
        assert_eq!(checked.description, "For board decks");
        assert_eq!(checked.sections[0].title, "Overview");
    }

    #[test]
    fn a_template_needs_a_name_and_a_section() {
        let nameless = TemplateContent {
            name: "  ".into(),
            ..content(vec![section("Overview")])
        };

        assert_eq!(
            nameless.validated().unwrap_err().to_string(),
            "name cannot be empty"
        );
        assert_eq!(
            content(Vec::new()).validated().unwrap_err().to_string(),
            "sections need at least one section"
        );
    }

    #[test]
    fn section_problems_name_the_section() {
        let repeated = content(vec![section("Overview"), section("overview")]);
        let untitled = content(vec![section("Overview"), section(" ")]);

        assert_eq!(
            repeated.validated().unwrap_err().to_string(),
            "section 2 repeats an earlier title"
        );
        assert_eq!(
            untitled.validated().unwrap_err().to_string(),
            "section 2 needs a title"
        );
    }

    #[test]
    fn content_round_trips_through_json_with_optional_fields_defaulted() {
        let json = r#"{ "name": "Mine", "sections": [{ "title": "Notes", "format": "todo" }] }"#;

        let parsed: TemplateContent = serde_json::from_str(json).unwrap();

        assert_eq!(parsed.sections[0].format, SectionFormat::Todo);
        assert!(parsed.auto_rules.is_empty());
        assert!(parsed.validated().is_ok());
    }
}
