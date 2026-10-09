//! Validated tag input: names, descriptions, and the edits the UI sends.

use deino_domain::PaletteColor;
use serde::Deserialize;

use super::TagError;

/// The longest tag name, in characters: a pill, not a sentence.
pub const MAX_TAG_NAME_CHARS: usize = 40;
/// The longest tag description, in characters. The auto-tagger reads every
/// description in one prompt, so they stay short.
pub const MAX_TAG_DESCRIPTION_CHARS: usize = 500;

/// A tag to create.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewTag {
    /// Its name; unique, ignoring case.
    pub name: String,
    /// Its colour; `None` picks the next palette slot.
    pub color: Option<PaletteColor>,
    /// When the tag applies, for the auto-tagger.
    pub description: String,
}

/// A tag's editable fields, all replaced at once.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TagUpdate {
    /// Its name; unique, ignoring case.
    pub name: String,
    /// Its colour.
    pub color: PaletteColor,
    /// When the tag applies, for the auto-tagger.
    pub description: String,
    /// True when the auto-tagger may apply it.
    pub auto_apply: bool,
}

/// A tag name: trimmed, non-empty, at most [`MAX_TAG_NAME_CHARS`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagName(String);

impl TagName {
    /// Validates user input as a tag name.
    pub fn parse(input: &str) -> Result<Self, TagError> {
        let name = input.trim();
        if name.is_empty() {
            return Err(TagError::EmptyName);
        }
        if name.chars().count() > MAX_TAG_NAME_CHARS {
            return Err(TagError::NameTooLong);
        }
        Ok(Self(name.to_owned()))
    }

    /// The validated name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A tag description: trimmed, at most [`MAX_TAG_DESCRIPTION_CHARS`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagDescription(String);

impl TagDescription {
    /// Validates user input as a tag description.
    pub fn parse(input: &str) -> Result<Self, TagError> {
        let description = input.trim();
        if description.chars().count() > MAX_TAG_DESCRIPTION_CHARS {
            return Err(TagError::DescriptionTooLong);
        }
        Ok(Self(description.to_owned()))
    }

    /// The validated description.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_non_empty_and_bounded_in_characters() {
        assert_eq!(TagName::parse("  Customer ").unwrap().as_str(), "Customer");
        assert!(matches!(TagName::parse("\t"), Err(TagError::EmptyName)));
        assert!(TagName::parse(&"ü".repeat(MAX_TAG_NAME_CHARS)).is_ok());
        assert!(matches!(
            TagName::parse(&"ü".repeat(MAX_TAG_NAME_CHARS + 1)),
            Err(TagError::NameTooLong)
        ));
    }

    #[test]
    fn descriptions_are_trimmed_and_bounded() {
        assert_eq!(
            TagDescription::parse(" Sales calls ").unwrap().as_str(),
            "Sales calls"
        );
        assert!(matches!(
            TagDescription::parse(&"x".repeat(MAX_TAG_DESCRIPTION_CHARS + 1)),
            Err(TagError::DescriptionTooLong)
        ));
    }

    #[test]
    fn a_new_tag_arrives_in_camel_case_with_an_optional_colour() {
        let new: NewTag =
            serde_json::from_str(r#"{ "name": "Hiring", "color": null, "description": "" }"#)
                .unwrap();
        assert_eq!(new.color, None);
        let update: TagUpdate = serde_json::from_str(
            r#"{ "name": "Hiring", "color": 3, "description": "", "autoApply": true }"#,
        )
        .unwrap();
        assert!(update.auto_apply);
    }
}
