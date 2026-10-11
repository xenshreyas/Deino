//! The vocabulary's types: entries as the user keeps them, validated drafts,
//! and the corrections made to transcripts.

use chrono::{DateTime, Utc};
use deino_domain::{SegmentId, VocabKind, VocabTermId};
use serde::{Deserialize, Serialize};

use super::VocabError;
use crate::segments::WordTiming;

/// The longest entry or variant, in characters.
pub const MAX_TERM_CHARS: usize = 80;

/// The most sounds-like variants one entry keeps.
pub const MAX_VARIANTS: usize = 12;

/// A vocabulary entry: a term spelled as given, or a replacement rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VocabTerm {
    /// The entry's ID.
    pub id: VocabTermId,
    /// A term, or a replacement rule.
    pub kind: VocabKind,
    /// How it is spelled: what its variants become.
    pub text: String,
    /// What the engine writes instead ("Sreyas" for "Shreyas"), in the
    /// order the user gave them.
    pub sounds_like: Vec<String>,
    /// When it was added.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
}

/// An entry as the user typed it, before validation.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TermInput {
    /// A term, or a replacement rule.
    pub kind: VocabKind,
    /// The spelling.
    pub text: String,
    /// What it sounds like when misheard.
    pub sounds_like: Vec<String>,
}

/// An entry that passed validation: trimmed, non-empty, within the limits,
/// variants unique (ignoring case) and different from the spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermDraft {
    /// A term, or a replacement rule.
    pub kind: VocabKind,
    /// The spelling.
    pub text: String,
    /// The variants.
    pub sounds_like: Vec<String>,
}

impl TermDraft {
    /// Validates what the user typed.
    pub fn parse(input: &TermInput) -> Result<Self, VocabError> {
        let text = phrase(&input.text)?;
        let mut sounds_like: Vec<String> = Vec::new();
        for variant in &input.sounds_like {
            let variant = phrase(variant)?;
            let repeated = sounds_like.iter().any(|kept| same(kept, &variant));
            if !repeated && !same(&variant, &text) {
                sounds_like.push(variant);
            }
        }
        if sounds_like.len() > MAX_VARIANTS {
            return Err(VocabError::TooManyVariants);
        }
        if input.kind == VocabKind::Replacement && sounds_like.is_empty() {
            return Err(VocabError::NothingToReplace);
        }
        Ok(Self {
            kind: input.kind,
            text,
            sounds_like,
        })
    }
}

/// A trimmed phrase with single spaces, within [`MAX_TERM_CHARS`].
fn phrase(input: &str) -> Result<String, VocabError> {
    let words: Vec<&str> = input.split_whitespace().collect();
    if words.is_empty() {
        return Err(VocabError::EmptyTerm);
    }
    let phrase = words.join(" ");
    if phrase.chars().count() > MAX_TERM_CHARS {
        return Err(VocabError::TermTooLong);
    }
    Ok(phrase)
}

fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// A segment's text and words as the engine wrote them, and what the
/// vocabulary made of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Correction {
    /// The corrected segment.
    pub segment_id: SegmentId,
    /// The engine's text.
    pub original_text: String,
    /// The engine's words.
    pub original_words: Vec<WordTiming>,
    /// The text after correction.
    pub corrected_text: String,
    /// The words after correction.
    pub corrected_words: Vec<WordTiming>,
}

/// One correction of a meeting, as the UI lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CorrectedSegment {
    /// The segment.
    pub segment_id: SegmentId,
    /// What the engine wrote.
    pub original_text: String,
    /// What the vocabulary made of it.
    pub corrected_text: String,
    /// False once the segment was edited since: undoing would lose the edit.
    pub undoable: bool,
}

/// What undoing a meeting's corrections did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UndoReport {
    /// Segments back to the engine's text.
    pub restored: u32,
    /// Segments left alone because they were edited after the correction.
    pub kept: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(kind: VocabKind, text: &str, sounds_like: &[&str]) -> TermInput {
        TermInput {
            kind,
            text: text.into(),
            sounds_like: sounds_like.iter().map(|&s| s.to_owned()).collect(),
        }
    }

    #[test]
    fn drafts_are_trimmed_and_drop_repeated_or_redundant_variants() {
        let draft = TermDraft::parse(&input(
            VocabKind::Term,
            "  Shreyas   Patel ",
            &[
                "Sreyas  Patel",
                "sreyas patel",
                "SHREYAS PATEL",
                " Shrey as ",
            ],
        ))
        .unwrap();

        assert_eq!(draft.text, "Shreyas Patel");
        assert_eq!(draft.sounds_like, ["Sreyas Patel", "Shrey as"]);
    }

    #[test]
    fn blank_overlong_and_empty_replacements_are_rejected() {
        let long = "a".repeat(MAX_TERM_CHARS + 1);
        let many: Vec<String> = (0..=MAX_VARIANTS).map(|i| format!("v{i}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();

        let parse =
            |kind, text: &str, variants: &[&str]| TermDraft::parse(&input(kind, text, variants));
        assert!(matches!(
            parse(VocabKind::Term, " ", &[]),
            Err(VocabError::EmptyTerm)
        ));
        assert!(matches!(
            parse(VocabKind::Term, &long, &[]),
            Err(VocabError::TermTooLong)
        ));
        assert!(matches!(
            parse(VocabKind::Term, "x", &[" "]),
            Err(VocabError::EmptyTerm)
        ));
        assert!(matches!(
            parse(VocabKind::Term, "x", &many),
            Err(VocabError::TooManyVariants)
        ));
        assert!(matches!(
            parse(VocabKind::Replacement, "going to", &[]),
            Err(VocabError::NothingToReplace)
        ));
    }
}
