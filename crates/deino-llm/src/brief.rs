//! The `brief` route's wire types (NOT-08): the structured answer the brief
//! prompt asks for, and the reference scheme its sources are numbered with.
//!
//! Sources are shown to the model as `[m1]`, `[a2]`, `[p1]`, `[c1]` and
//! `[e1]` blocks; a bullet cites them by those references. These are wire
//! types: `deino_core::briefs` resolves the references against what it
//! showed, and the eval harness scores the answers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What a source is, by the letter its reference starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceKind {
    /// An earlier meeting (`m`).
    Meeting,
    /// An open action item (`a`).
    ActionItem,
    /// What the user knows about a person (`p`).
    Person,
    /// What the user knows about a company (`c`).
    Company,
    /// The calendar invitation (`e`).
    Event,
}

impl SourceKind {
    /// The reference's letter.
    pub const fn letter(self) -> char {
        match self {
            Self::Meeting => 'm',
            Self::ActionItem => 'a',
            Self::Person => 'p',
            Self::Company => 'c',
            Self::Event => 'e',
        }
    }

    fn from_letter(letter: char) -> Option<Self> {
        Some(match letter {
            'm' => Self::Meeting,
            'a' => Self::ActionItem,
            'p' => Self::Person,
            'c' => Self::Company,
            'e' => Self::Event,
            _ => return None,
        })
    }
}

/// A source's reference: its kind and its 1-based number (`m2`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRef {
    /// What it is.
    pub kind: SourceKind,
    /// Its number among the sources of its kind, from 1.
    pub number: u32,
}

impl SourceRef {
    /// The reference as the prompt writes it, `m2`.
    pub fn label(self) -> String {
        format!("{}{}", self.kind.letter(), self.number)
    }

    /// The reference a model wrote: `m2`, `M2`, `[m2]` or `m2 (Kickoff)`
    /// all name meeting 2; anything else is `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim().trim_start_matches(['[', '(']);
        let mut chars = trimmed.chars();
        let kind = SourceKind::from_letter(chars.next()?.to_ascii_lowercase())?;
        let digits: String = chars.take_while(char::is_ascii_digit).collect();
        let number = digits.parse().ok().filter(|number| *number > 0)?;
        Some(Self { kind, number })
    }
}

/// One source block of the prompt: `[m1] Heading`, then the body.
pub fn source_block(reference: SourceRef, heading: &str, body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        return format!("[{}] {heading}", reference.label());
    }
    format!("[{}] {heading}\n{body}", reference.label())
}

/// `brief`: the bullets of a pre-meeting brief, or that there is too little
/// to write one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BriefAnswer {
    /// True when the sources say too little for two useful bullets (for
    /// example, a first meeting with someone new); `bullets` is then empty.
    pub insufficient: bool,
    /// Two to four bullets, the most useful first.
    pub bullets: Vec<BriefAnswerBullet>,
}

/// One bullet of a brief, as the model wrote it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BriefAnswerBullet {
    /// One plain-text sentence of at most 30 words.
    pub text: String,
    /// The references of the sources it rests on, such as `m1` or `a2`.
    pub sources: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_parse_in_the_forms_models_write_them() {
        let meeting_two = SourceRef {
            kind: SourceKind::Meeting,
            number: 2,
        };
        for text in ["m2", "M2", "[m2]", " m2 (Kickoff)", "(m2)"] {
            assert_eq!(SourceRef::parse(text), Some(meeting_two), "{text}");
        }
        for text in ["", "x2", "m", "m0", "meeting"] {
            assert_eq!(SourceRef::parse(text), None, "{text}");
        }
        assert_eq!(meeting_two.label(), "m2");
    }

    #[test]
    fn a_source_block_is_the_reference_heading_and_body() {
        let reference = SourceRef {
            kind: SourceKind::Person,
            number: 1,
        };

        assert_eq!(
            source_block(reference, "Maya Chen", " Leads infra. "),
            "[p1] Maya Chen\nLeads infra."
        );
        assert_eq!(source_block(reference, "Maya Chen", ""), "[p1] Maya Chen");
    }
}
