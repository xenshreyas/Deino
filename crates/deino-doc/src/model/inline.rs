//! Inline nodes and marks: text, line breaks, people, transcript citations,
//! and the formatting and provenance laid over them.

use serde::{Deserialize, Serialize};

use super::{CitationAttrs, LinkAttrs, MentionAttrs, OriginAttrs};

/// A node inside a paragraph, heading or code block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Inline {
    /// A run of text with the same marks. Never empty.
    Text {
        /// The characters.
        text: String,
        /// Formatting and provenance, outermost first (see [`Mark::rank`]).
        #[serde(default)]
        marks: Vec<Mark>,
    },
    /// A line break inside a block (⇧↩ in the editor).
    HardBreak {
        /// The marks the break carries, so provenance spans it.
        #[serde(default)]
        marks: Vec<Mark>,
    },
    /// A person, shown as a chip (`@Priya`).
    Mention {
        /// Who is mentioned.
        attrs: MentionAttrs,
        /// Marks over the chip, such as its origin.
        #[serde(default)]
        marks: Vec<Mark>,
    },
    /// Evidence for the text before it: a moment of the transcript (NOT-02/NOT-03).
    Citation {
        /// The cited transcript segment.
        attrs: CitationAttrs,
        /// Marks over the citation, such as its origin.
        #[serde(default)]
        marks: Vec<Mark>,
    },
}

/// Formatting or provenance over inline content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Mark {
    /// Who wrote the text: the user, or the AI (NOT-02 renders AI text grey).
    Origin {
        /// The author, and whether the user edited AI text.
        attrs: OriginAttrs,
    },
    /// A hyperlink.
    Link {
        /// Where it points.
        attrs: LinkAttrs,
    },
    /// Bold (`**`).
    Bold,
    /// Italic (`*`).
    Italic,
    /// Underline; Markdown has none, so it is written as `<u>`.
    Underline,
    /// Strikethrough (`~~`).
    Strike,
    /// Inline code; combines with [`Mark::Origin`] only.
    Code,
}

impl Inline {
    /// A run of `text` with `marks`.
    pub fn text(text: impl Into<String>, marks: Vec<Mark>) -> Self {
        Self::Text {
            text: text.into(),
            marks,
        }
    }

    /// The node's marks.
    pub fn marks(&self) -> &[Mark] {
        match self {
            Self::Text { marks, .. }
            | Self::HardBreak { marks }
            | Self::Mention { marks, .. }
            | Self::Citation { marks, .. } => marks,
        }
    }

    /// True for whitespace-only text and line breaks.
    pub fn is_blank(&self) -> bool {
        match self {
            Self::Text { text, .. } => text.trim().is_empty(),
            Self::HardBreak { .. } => true,
            Self::Mention { .. } | Self::Citation { .. } => false,
        }
    }
}

impl Mark {
    /// The nesting order: lower ranks wrap higher ones. Provenance is the
    /// outermost, so one origin span covers a whole run of formatted text;
    /// links wrap emphasis so a link keeps one target across bold and plain
    /// words; code is innermost. Marks are stored sorted by rank.
    pub fn rank(&self) -> u8 {
        match self {
            Self::Origin { .. } => 0,
            Self::Link { .. } => 1,
            Self::Bold => 2,
            Self::Italic => 3,
            Self::Underline => 4,
            Self::Strike => 5,
            Self::Code => 6,
        }
    }

    /// Whether two marks are the same kind, whatever their attributes.
    pub fn same_kind(&self, other: &Self) -> bool {
        self.rank() == other.rank()
    }
}
