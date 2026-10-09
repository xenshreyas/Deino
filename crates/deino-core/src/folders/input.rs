//! Validated folder input: names, icons and descriptions.
//!
//! Each type can only be built by `parse`, so a service holding one knows
//! the value passed the rules below.

use super::FolderError;

/// The longest folder name, in characters.
pub const MAX_FOLDER_NAME_CHARS: usize = 80;
/// The longest folder description, in characters. Chat (AI-02) reads it as
/// context, so it is a paragraph or two, not a document.
pub const MAX_FOLDER_DESCRIPTION_CHARS: usize = 2_000;
/// The longest icon name, in characters.
pub const MAX_FOLDER_ICON_CHARS: usize = 32;

/// A folder name: trimmed, non-empty, at most [`MAX_FOLDER_NAME_CHARS`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderName(String);

impl FolderName {
    /// Validates user input as a folder name.
    pub fn parse(input: &str) -> Result<Self, FolderError> {
        let name = input.trim();
        if name.is_empty() {
            return Err(FolderError::EmptyName);
        }
        if name.chars().count() > MAX_FOLDER_NAME_CHARS {
            return Err(FolderError::NameTooLong);
        }
        Ok(Self(name.to_owned()))
    }

    /// The validated name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An icon name from the UI's icon set (`briefcase`, `users`, ...): lower
/// case ASCII letters, digits and dashes. The UI shows the default folder
/// icon for a name it does not know, so the set can grow without a migration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderIcon(String);

impl FolderIcon {
    /// Validates an icon name.
    pub fn parse(input: &str) -> Result<Self, FolderError> {
        let name = input.trim();
        let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
        let valid = !name.is_empty()
            && name.len() <= MAX_FOLDER_ICON_CHARS
            && name.chars().all(allowed)
            && !name.starts_with('-');
        if valid {
            Ok(Self(name.to_owned()))
        } else {
            Err(FolderError::InvalidIcon)
        }
    }

    /// The validated icon name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A folder description: trimmed, at most [`MAX_FOLDER_DESCRIPTION_CHARS`];
/// empty means none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderDescription(String);

impl FolderDescription {
    /// Validates user input as a folder description.
    pub fn parse(input: &str) -> Result<Self, FolderError> {
        let description = input.trim();
        if description.chars().count() > MAX_FOLDER_DESCRIPTION_CHARS {
            return Err(FolderError::DescriptionTooLong);
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
        assert_eq!(FolderName::parse("  Clients ").unwrap().as_str(), "Clients");
        assert!(matches!(
            FolderName::parse(" \n"),
            Err(FolderError::EmptyName)
        ));
        assert!(FolderName::parse(&"é".repeat(MAX_FOLDER_NAME_CHARS)).is_ok());
        assert!(matches!(
            FolderName::parse(&"é".repeat(MAX_FOLDER_NAME_CHARS + 1)),
            Err(FolderError::NameTooLong)
        ));
    }

    #[test]
    fn icons_are_lower_case_kebab_names() {
        assert_eq!(
            FolderIcon::parse(" briefcase ").unwrap().as_str(),
            "briefcase"
        );
        assert!(FolderIcon::parse("book-open-2").is_ok());
        for invalid in ["", "Briefcase", "brief case", "-x", "../x", &"a".repeat(33)] {
            assert!(
                matches!(FolderIcon::parse(invalid), Err(FolderError::InvalidIcon)),
                "{invalid:?}"
            );
        }
    }

    #[test]
    fn descriptions_may_be_empty_but_not_overlong() {
        assert_eq!(FolderDescription::parse("  ").unwrap().as_str(), "");
        assert!(matches!(
            FolderDescription::parse(&"x".repeat(MAX_FOLDER_DESCRIPTION_CHARS + 1)),
            Err(FolderError::DescriptionTooLong)
        ));
    }
}
