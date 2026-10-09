//! The meeting as the meetings feature sees it.

use chrono::{DateTime, Utc};
use deino_domain::MeetingId;
use serde::Serialize;

use super::MeetingError;

/// The longest title a user can give a meeting, in characters.
pub const MAX_TITLE_CHARS: usize = 200;

/// A meeting, as the list and the meeting page show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    /// The meeting's ID.
    pub id: MeetingId,
    /// The title shown in the sidebar and the meeting header.
    pub title: String,
    /// When the meeting was saved.
    pub created_at: DateTime<Utc>,
    /// When the meeting, its title or its summary last changed.
    pub updated_at: DateTime<Utc>,
}

/// A title that passed validation: trimmed, non-empty and at most
/// [`MAX_TITLE_CHARS`] characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingTitle(String);

impl MeetingTitle {
    /// Validates user input as a meeting title.
    pub fn parse(input: &str) -> Result<Self, MeetingError> {
        let title = input.trim();
        if title.is_empty() {
            return Err(MeetingError::EmptyTitle);
        }
        if title.chars().count() > MAX_TITLE_CHARS {
            return Err(MeetingError::TitleTooLong);
        }
        Ok(Self(title.to_owned()))
    }

    /// The validated title.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_trimmed() {
        assert_eq!(
            MeetingTitle::parse("  Weekly sync \n").unwrap().as_str(),
            "Weekly sync"
        );
    }

    #[test]
    fn blank_titles_are_rejected() {
        assert!(matches!(
            MeetingTitle::parse(" \t "),
            Err(MeetingError::EmptyTitle)
        ));
    }

    #[test]
    fn the_length_limit_counts_characters_not_bytes() {
        let longest = "é".repeat(MAX_TITLE_CHARS);
        assert!(MeetingTitle::parse(&longest).is_ok());

        let too_long = "é".repeat(MAX_TITLE_CHARS + 1);
        assert!(matches!(
            MeetingTitle::parse(&too_long),
            Err(MeetingError::TitleTooLong)
        ));
    }

    #[test]
    fn meetings_serialize_with_camel_case_keys() {
        let meeting = Meeting {
            id: MeetingId::new("meeting-1"),
            title: "Standup".into(),
            created_at: DateTime::UNIX_EPOCH,
            updated_at: DateTime::UNIX_EPOCH,
        };

        let json = serde_json::to_value(&meeting).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "id": "meeting-1",
                "title": "Standup",
                "createdAt": "1970-01-01T00:00:00Z",
                "updatedAt": "1970-01-01T00:00:00Z",
            })
        );
    }
}
