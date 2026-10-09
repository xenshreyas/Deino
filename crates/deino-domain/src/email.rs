//! [`Email`]: a normalised email address.

use std::fmt;

use serde::{Deserialize, Serialize};

/// An email address in canonical form: lower case, no surrounding
/// whitespace, one `@` between a non-empty local part and a dotted domain.
///
/// [`Email::parse`] normalises user input into this form. Deserializing and
/// [`TryFrom<String>`] accept only the canonical form and never rewrite a
/// value, so stored and fixture data are checked rather than silently
/// changed. Stored as TEXT; `people.email` is UNIQUE, which the lower case
/// makes case-insensitive.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    specta::Type,
    sqlx::Type,
)]
#[serde(try_from = "String", into = "String")]
#[sqlx(transparent)]
pub struct Email(String);

/// A string that is not an email address in canonical form.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a lower-case email address")]
pub struct InvalidEmail(pub String);

impl Email {
    /// Normalises user input (trims it, lower-cases it) into an address.
    pub fn parse(input: &str) -> Result<Self, InvalidEmail> {
        Self::try_from(input.trim().to_lowercase())
    }

    /// The address as stored.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The part after the `@`, which identifies the person's company.
    pub fn domain(&self) -> &str {
        self.0.split_once('@').map_or("", |(_, domain)| domain)
    }
}

impl TryFrom<String> for Email {
    type Error = InvalidEmail;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let shaped = value.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !domain.contains('@')
        });
        let canonical = value == value.to_lowercase() && !value.contains(char::is_whitespace);
        if shaped && canonical {
            Ok(Self(value))
        } else {
            Err(InvalidEmail(value))
        }
    }
}

impl From<Email> for String {
    fn from(email: Email) -> Self {
        email.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_normalises_user_input() {
        let email = Email::parse(" Ada@Example.COM ").unwrap();
        assert_eq!(email.as_str(), "ada@example.com");
        assert_eq!(email.domain(), "example.com");
    }

    #[test]
    fn only_canonical_addresses_convert_or_deserialize() {
        let canonical = Email::try_from("maya@deino.dev".to_owned()).unwrap();
        assert_eq!(canonical.to_string(), "maya@deino.dev");

        for invalid in [
            "Maya@deino.dev",
            "maya@deino",
            "@deino.dev",
            "maya",
            " maya@deino.dev",
        ] {
            assert!(Email::try_from(invalid.to_owned()).is_err(), "{invalid:?}");
        }
        assert!(serde_json::from_str::<Email>(r#""MAYA@deino.dev""#).is_err());
    }

    #[test]
    fn things_that_are_not_addresses_do_not_parse() {
        for input in [
            "",
            "ada",
            "ada@",
            "a@b@c.com",
            "ada lovelace@example.com",
            "ada@.com",
        ] {
            assert!(Email::parse(input).is_err(), "{input:?}");
        }
    }
}
