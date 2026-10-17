//! Speaker tags (CAP-05): names read from a meeting app's window.
//!
//! While Deino records, the meeting app's Accessibility tree says who is in
//! the call and whose tile shows the active-speaker indicator. The watcher
//! (`deino-speaker-tags`) turns that into [`TagSpan`]s on the recording's
//! timeline, and `identify_speakers` (`deino-core::speaker_tags`) uses them
//! to name the system channel's segments before voices are matched.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Millis;

/// The longest display name kept; longer text is not a participant's name.
pub const MAX_DISPLAY_NAME_CHARS: usize = 100;

db_enum! {
    /// The meeting app a speaker tag was read from. Stored in
    /// `speaker_tag_spans.app`.
    CallApp {
        /// The Zoom desktop app (`us.zoom.xos`).
        Zoom = "zoom",
        /// The Microsoft Teams desktop app.
        Teams = "teams",
        /// Google Meet in Chrome or a Chromium browser (Brave, Edge, Arc, Vivaldi).
        Meet = "meet",
    }
}

/// A participant's name as the meeting app shows it ("Alex Chen"), without
/// the app's role suffixes. Whitespace is collapsed; empty text, control
/// characters and anything over [`MAX_DISPLAY_NAME_CHARS`] are rejected.
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
pub struct DisplayName(String);

/// Text that cannot be a participant's display name.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("not a display name ({0} characters)")]
pub struct InvalidDisplayName(pub usize);

impl DisplayName {
    /// Normalises text read from a window: trims it and collapses runs of
    /// whitespace into one space.
    pub fn parse(raw: &str) -> Result<Self, InvalidDisplayName> {
        Self::try_from(raw.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    /// The name as shown.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The name folded for comparison: "ALEX chen" and "Alex Chen" match.
    pub fn key(&self) -> String {
        self.0.to_lowercase()
    }

    /// Whether `other` is the same name, ignoring case and spacing.
    pub fn matches(&self, other: &str) -> bool {
        Self::parse(other).is_ok_and(|other| other.key() == self.key())
    }
}

impl TryFrom<String> for DisplayName {
    type Error = InvalidDisplayName;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let length = value.chars().count();
        let valid = length > 0
            && length <= MAX_DISPLAY_NAME_CHARS
            && value.trim() == value
            && !value.contains("  ")
            && !value.chars().any(char::is_control);
        if valid {
            Ok(Self(value))
        } else {
            Err(InvalidDisplayName(length))
        }
    }
}

impl From<DisplayName> for String {
    fn from(name: DisplayName) -> Self {
        name.0
    }
}

impl fmt::Display for DisplayName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One stretch of a recording during which a participant's tile showed the
/// active-speaker indicator. Times are offsets from the start of the
/// recording on its own timeline (pauses skipped), like segments'.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagSpan {
    /// When the indicator lit up.
    pub start: Millis,
    /// When it moved on or went dark; after `start`.
    pub end: Millis,
    /// Whose tile it was.
    pub name: DisplayName,
    /// The app it was read from.
    pub app: CallApp,
}

impl TagSpan {
    /// How long the span lasts.
    pub fn length(&self) -> u32 {
        self.end.0.saturating_sub(self.start.0)
    }

    /// How much of `start..end` the span covers.
    pub fn overlap(&self, start: Millis, end: Millis) -> u32 {
        let from = self.start.max(start);
        let to = self.end.min(end);
        to.0.saturating_sub(from.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_enum::assert_names_agree;

    fn name(text: &str) -> DisplayName {
        DisplayName::parse(text).unwrap()
    }

    #[test]
    fn apps_serialize_as_their_schema_names() {
        assert_names_agree(CallApp::ALL, CallApp::as_str);
    }

    #[test]
    fn parsing_trims_and_collapses_whitespace() {
        assert_eq!(name("  Alex \n Chen ").as_str(), "Alex Chen");
        assert!(name("Alex Chen").matches("alex   CHEN"));
        assert!(!name("Alex Chen").matches("Alex"));
    }

    #[test]
    fn blank_oversized_or_control_text_is_rejected() {
        let long = "a".repeat(MAX_DISPLAY_NAME_CHARS + 1);
        for invalid in ["", "   ", "Alex\u{0}Chen", long.as_str()] {
            assert!(DisplayName::parse(invalid).is_err(), "{invalid:?}");
        }
        for unnormalised in [" Alex", "Alex  Chen"] {
            assert!(DisplayName::try_from(unnormalised.to_owned()).is_err());
        }
        assert!(serde_json::from_str::<DisplayName>(r#"" ""#).is_err());
    }

    #[test]
    fn a_span_measures_its_overlap_with_a_stretch() {
        let span = TagSpan {
            start: Millis(1_000),
            end: Millis(3_000),
            name: name("Alex"),
            app: CallApp::Zoom,
        };

        assert_eq!(span.length(), 2_000);
        assert_eq!(span.overlap(Millis(2_000), Millis(5_000)), 1_000);
        assert_eq!(span.overlap(Millis(0), Millis(500)), 0);
        assert_eq!(span.overlap(Millis(0), Millis(9_000)), 2_000);
    }
}
