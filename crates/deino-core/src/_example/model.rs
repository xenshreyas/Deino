//! The example's types. Validation happens once, in constructors, so the
//! rest of the feature can rely on the invariants.

use chrono::{DateTime, Utc};
use deino_domain::ExampleNoteId;
use serde::Serialize;

use super::ExampleError;

/// The longest note text, in characters.
pub const MAX_NOTE_CHARS: usize = 500;

/// A saved note.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExampleNote {
    /// The note's ID.
    pub id: ExampleNoteId,
    /// The note's text, trimmed and non-empty.
    pub text: String,
    /// When the note was saved.
    pub created_at: DateTime<Utc>,
}

/// Note text that passed validation: trimmed, non-empty and at most
/// [`MAX_NOTE_CHARS`] characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteText(String);

impl NoteText {
    /// Validates user input as note text.
    pub fn parse(input: &str) -> Result<Self, ExampleError> {
        let text = input.trim();
        if text.is_empty() {
            return Err(ExampleError::EmptyText);
        }
        if text.chars().count() > MAX_NOTE_CHARS {
            return Err(ExampleError::TextTooLong);
        }
        Ok(Self(text.to_owned()))
    }

    /// The validated text.
    pub fn into_inner(self) -> String {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_trimmed() {
        assert_eq!(
            NoteText::parse("  buy milk ").unwrap().into_inner(),
            "buy milk"
        );
    }

    #[test]
    fn blank_and_overlong_text_is_rejected() {
        assert!(matches!(
            NoteText::parse("  "),
            Err(ExampleError::EmptyText)
        ));
        let overlong = "a".repeat(MAX_NOTE_CHARS + 1);
        assert!(matches!(
            NoteText::parse(&overlong),
            Err(ExampleError::TextTooLong)
        ));
    }
}
