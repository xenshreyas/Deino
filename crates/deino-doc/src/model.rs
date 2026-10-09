//! The document model: TipTap's JSON, as Rust types.
//!
//! A [`Doc`] is exactly what the editor's `getJSON()` produces and
//! `setContent()` accepts, so it can be stored as-is (`meeting_notes.notes_json`)
//! and sent over IPC without a translation layer. The types split ProseMirror's
//! one node kind into [`Block`] and [`Inline`], so "a paragraph holds only
//! inline content" is enforced by the compiler; what the types cannot say
//! (lists are non-empty, items start with a paragraph, text is non-empty,
//! which marks may combine) is checked by [`validate`](crate::validate).
//!
//! Every type serializes with TipTap's names (`bulletList`, `taskItem`,
//! `hardBreak`) and camelCase attributes, and ignores attributes it does not
//! model (`link.target`, `image.width`, `mention.mentionSuggestionChar`), so
//! the editor's defaults never fail a save. TipTap omits empty `content` and
//! `marks`; Deino reads either form and always writes `[]`, so the generated
//! TypeScript types are exact.

mod attrs;
mod inline;

pub use attrs::{
    Author, CitationAttrs, CodeBlockAttrs, HeadingAttrs, ImageAttrs, LinkAttrs, MentionAttrs,
    OrderedListAttrs, OriginAttrs, TaskItemAttrs,
};
pub use inline::{Inline, Mark};
use serde::{Deserialize, Serialize};

/// A whole document: one or more blocks (`{"type": "doc", "content": [...]}`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename = "doc")]
pub struct Doc {
    /// The blocks, top to bottom.
    pub content: Vec<Block>,
}

/// A block-level node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Block {
    /// A paragraph; an empty one is a blank line in the editor.
    Paragraph {
        /// The paragraph's text and inline nodes.
        #[serde(default)]
        content: Vec<Inline>,
    },
    /// A heading, level 1 to 6.
    Heading {
        /// Its level.
        attrs: HeadingAttrs,
        /// The heading's text and inline nodes.
        #[serde(default)]
        content: Vec<Inline>,
    },
    /// A bulleted list.
    BulletList {
        /// Its items.
        content: Vec<ListItem>,
    },
    /// A numbered list.
    OrderedList {
        /// Where the numbering starts.
        #[serde(default)]
        attrs: OrderedListAttrs,
        /// Its items.
        content: Vec<ListItem>,
    },
    /// A checklist.
    TaskList {
        /// Its items.
        content: Vec<TaskItem>,
    },
    /// A quotation.
    Blockquote {
        /// The quoted blocks.
        content: Vec<Block>,
    },
    /// Preformatted code; its content is unmarked text only.
    CodeBlock {
        /// The code's language, if known.
        #[serde(default)]
        attrs: CodeBlockAttrs,
        /// The code.
        #[serde(default)]
        content: Vec<Inline>,
    },
    /// A horizontal divider.
    HorizontalRule,
    /// An image, by reference (a note's images live in its meeting folder).
    Image {
        /// Where the image is and how to describe it.
        attrs: ImageAttrs,
    },
}

/// An item of a bulleted or numbered list (`{"type": "listItem"}`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename = "listItem")]
pub struct ListItem {
    /// A paragraph, then any nested blocks.
    pub content: Vec<Block>,
}

/// An item of a checklist (`{"type": "taskItem"}`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename = "taskItem")]
pub struct TaskItem {
    /// Whether it is ticked.
    pub attrs: TaskItemAttrs,
    /// A paragraph, then any nested blocks.
    pub content: Vec<Block>,
}

impl Doc {
    /// The document the editor starts from: one empty paragraph.
    pub fn empty() -> Self {
        Self {
            content: vec![Block::Paragraph { content: vec![] }],
        }
    }

    /// True when the document has no text, mention, citation or image.
    pub fn is_blank(&self) -> bool {
        self.content.iter().all(Block::is_blank)
    }

    /// Every image in the document, in reading order.
    pub fn images(&self) -> Vec<&ImageAttrs> {
        let mut images = Vec::new();
        for block in &self.content {
            block.collect_images(&mut images);
        }
        images
    }
}

impl Default for Doc {
    fn default() -> Self {
        Self::empty()
    }
}

impl Block {
    /// A paragraph of `content`.
    pub fn paragraph(content: Vec<Inline>) -> Self {
        Self::Paragraph { content }
    }

    /// The blocks nested directly inside this one (list items' blocks included).
    pub fn children(&self) -> Vec<&Block> {
        match self {
            Self::BulletList { content } | Self::OrderedList { content, .. } => {
                content.iter().flat_map(|item| &item.content).collect()
            }
            Self::TaskList { content } => content.iter().flat_map(|item| &item.content).collect(),
            Self::Blockquote { content } => content.iter().collect(),
            _ => Vec::new(),
        }
    }

    /// The block's own inline content (empty for containers and atoms).
    pub fn inline_content(&self) -> &[Inline] {
        match self {
            Self::Paragraph { content }
            | Self::Heading { content, .. }
            | Self::CodeBlock { content, .. } => content,
            _ => &[],
        }
    }

    fn is_blank(&self) -> bool {
        match self {
            Self::Image { .. } => false,
            Self::HorizontalRule => true,
            _ => {
                self.inline_content().iter().all(Inline::is_blank)
                    && self.children().into_iter().all(Self::is_blank)
            }
        }
    }

    fn collect_images<'doc>(&'doc self, images: &mut Vec<&'doc ImageAttrs>) {
        if let Self::Image { attrs } = self {
            images.push(attrs);
        }
        for child in self.children() {
            child.collect_images(images);
        }
    }
}
