//! [`BundleId`]: the identifier of a macOS app (`us.zoom.xos`).

use std::fmt;

use serde::{Deserialize, Serialize};

/// The longest identifier accepted; Launch Services caps bundle IDs well below it.
const MAX_LEN: usize = 255;

/// A macOS bundle identifier, such as `com.tinyspeck.slackmacgap`.
///
/// Meeting detection names apps by it, and the user's "always ignore" list
/// stores it. A process without a bundle gets a synthetic identifier from
/// its executable name (`process.ffmpeg`), so every app that holds the
/// microphone can be ignored. Any non-empty text without whitespace or
/// control characters, up to 255 bytes, is accepted unchanged; comparisons
/// are exact, as the bundle tables use the identifiers apps ship with.
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(try_from = "String", into = "String")]
pub struct BundleId(String);

/// Text that cannot be a bundle identifier.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a bundle identifier")]
pub struct InvalidBundleId(pub String);

impl BundleId {
    /// The identifier as macOS reports it.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is a helper of `parent`: Chrome's audio helper is
    /// `com.google.Chrome.helper` (case differs between browsers).
    pub fn is_helper_of(&self, parent: &Self) -> bool {
        let (own, parent) = (self.0.to_ascii_lowercase(), parent.0.to_ascii_lowercase());
        own.strip_prefix(&parent)
            .is_some_and(|rest| rest.starts_with('.'))
    }
}

impl TryFrom<String> for BundleId {
    type Error = InvalidBundleId;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let valid = !value.is_empty()
            && value.len() <= MAX_LEN
            && !value.chars().any(|c| c.is_whitespace() || c.is_control());
        if valid {
            Ok(Self(value))
        } else {
            Err(InvalidBundleId(value))
        }
    }
}

impl TryFrom<&str> for BundleId {
    type Error = InvalidBundleId;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_owned())
    }
}

impl From<BundleId> for String {
    fn from(id: BundleId) -> Self {
        id.0
    }
}

impl fmt::Display for BundleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_identifiers_are_accepted_unchanged() {
        for id in ["us.zoom.xos", "com.tinyspeck.slackmacgap", "process.ffmpeg"] {
            assert_eq!(BundleId::try_from(id).unwrap().as_str(), id);
        }
    }

    #[test]
    fn blank_spaced_or_oversized_text_is_rejected() {
        for invalid in ["", "com.apple Safari", "us.zoom\n", &"a".repeat(256)] {
            assert!(BundleId::try_from(invalid).is_err(), "{invalid:?}");
        }
        assert!(serde_json::from_str::<BundleId>(r#""""#).is_err());
    }

    #[test]
    fn helpers_match_their_parent_case_insensitively_and_only_after_a_dot() {
        let chrome = BundleId::try_from("com.google.Chrome").unwrap();
        let helper = BundleId::try_from("com.google.chrome.helper").unwrap();
        let canary = BundleId::try_from("com.google.ChromeCanary").unwrap();

        assert!(helper.is_helper_of(&chrome));
        assert!(!canary.is_helper_of(&chrome));
        assert!(!chrome.is_helper_of(&chrome));
    }
}
