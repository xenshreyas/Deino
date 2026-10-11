//! Notes as typed blocks: Markdown parsed once, walked by every rich format.
//!
//! Notes are stored as Markdown (the legacy summary, the notepad until
//! NOT-01 stores TipTap JSON). [`parse`] turns them into a small tree of
//! [`Block`]s and [`Inline`]s with CommonMark semantics plus task lists and
//! strikethrough, so the HTML, DOCX, PDF, text and Markdown renderers share
//! one reading of the notes instead of five regex passes. Raw HTML in notes
//! is kept as text, never interpreted.

mod parse;

use serde::Serialize;

pub use parse::parse;

/// A block of a note.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Block {
    /// A heading; `level` is 1 (largest) to 6.
    Heading {
        /// The heading's depth, 1 to 6.
        level: u8,
        /// Its text.
        content: Vec<Inline>,
    },
    /// A paragraph.
    Paragraph {
        /// Its text.
        content: Vec<Inline>,
    },
    /// A bulleted, numbered or task list.
    List {
        /// True for a numbered list.
        ordered: bool,
        /// Its items, in order.
        items: Vec<ListItem>,
    },
    /// A block quote.
    Quote {
        /// The quoted blocks.
        blocks: Vec<Block>,
    },
    /// Preformatted code.
    Code {
        /// The code, without its fences.
        text: String,
    },
    /// A thematic break.
    Rule,
}

/// One item of a [`Block::List`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    /// `Some(done)` for a task-list item (`- [ ]` / `- [x]`).
    pub checked: Option<bool>,
    /// The item's blocks: usually one paragraph, then any nested list.
    pub blocks: Vec<Block>,
}

/// A run of text inside a block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Inline {
    /// Plain text.
    Text {
        /// The text.
        text: String,
    },
    /// Bold text.
    Strong {
        /// What is bold.
        content: Vec<Inline>,
    },
    /// Italic text.
    Emphasis {
        /// What is italic.
        content: Vec<Inline>,
    },
    /// Struck-through text.
    Strike {
        /// What is struck through.
        content: Vec<Inline>,
    },
    /// Inline code.
    Code {
        /// The code.
        text: String,
    },
    /// A link.
    Link {
        /// Where it points.
        href: String,
        /// Its text.
        content: Vec<Inline>,
    },
    /// A hard line break.
    Break,
}

impl Inline {
    /// Plain text.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }
}

/// The text of `inlines` without formatting; breaks become spaces.
pub fn plain(inlines: &[Inline]) -> String {
    let mut out = String::new();
    push_plain(inlines, &mut out);
    out
}

fn push_plain(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text { text } | Inline::Code { text } => out.push_str(text),
            Inline::Strong { content }
            | Inline::Emphasis { content }
            | Inline::Strike { content }
            | Inline::Link { content, .. } => push_plain(content, out),
            Inline::Break => out.push(' '),
        }
    }
}

/// `blocks` with every heading moved down by `by` levels (capped at 6), so a
/// note's own `#` headings nest under the export's section headings.
pub fn demote(blocks: Vec<Block>, by: u8) -> Vec<Block> {
    blocks
        .into_iter()
        .map(|block| match block {
            Block::Heading { level, content } => Block::Heading {
                level: level.saturating_add(by).min(6),
                content,
            },
            Block::Quote { blocks } => Block::Quote {
                blocks: demote(blocks, by),
            },
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_drops_formatting_and_keeps_words() {
        let blocks = parse("**Ship** the _beta_ with `v2` [today](https://x.dev)");
        let Some(Block::Paragraph { content }) = blocks.first() else {
            panic!("a paragraph: {blocks:?}");
        };
        assert_eq!(plain(content), "Ship the beta with v2 today");
    }

    #[test]
    fn demoting_caps_headings_at_level_six() {
        let blocks = demote(parse("# Title\n\n##### Deep"), 2);
        let levels: Vec<u8> = blocks
            .iter()
            .filter_map(|block| match block {
                Block::Heading { level, .. } => Some(*level),
                _ => None,
            })
            .collect();
        assert_eq!(levels, [3, 6]);
    }
}
