//! The canonical form of a document, so equal documents compare equal.
//!
//! ProseMirror orders a node's marks by the editor's extension order, which
//! the Rust side cannot know, and an edited document may hold neighbouring
//! text runs with identical marks. [`Doc::normalize`] sorts marks by
//! [`Mark::rank`](crate::Mark::rank) and merges such runs. Stored notes are
//! normalized; [`from_markdown`](crate::from_markdown) already returns this form.

use crate::{Block, Doc, Inline, Mark};

impl Doc {
    /// Rewrites the document in canonical form.
    pub fn normalize(&mut self) {
        for block in &mut self.content {
            normalize_block(block);
        }
    }

    /// The document in canonical form.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.normalize();
        self
    }
}

fn normalize_block(block: &mut Block) {
    match block {
        Block::Paragraph { content }
        | Block::Heading { content, .. }
        | Block::CodeBlock { content, .. } => normalize_inlines(content),
        Block::BulletList { content } | Block::OrderedList { content, .. } => {
            for item in content {
                item.content.iter_mut().for_each(normalize_block);
            }
        }
        Block::TaskList { content } => {
            for item in content {
                item.content.iter_mut().for_each(normalize_block);
            }
        }
        Block::Blockquote { content } => content.iter_mut().for_each(normalize_block),
        Block::HorizontalRule | Block::Image { .. } => {}
    }
}

fn normalize_inlines(content: &mut Vec<Inline>) {
    let mut merged: Vec<Inline> = Vec::with_capacity(content.len());
    for mut inline in content.drain(..) {
        sort_marks(&mut inline);
        match (merged.last_mut(), inline) {
            (
                Some(Inline::Text { text, marks }),
                Inline::Text {
                    text: more,
                    marks: more_marks,
                },
            ) if *marks == more_marks => text.push_str(&more),
            (_, inline) => merged.push(inline),
        }
    }
    *content = merged;
}

fn sort_marks(inline: &mut Inline) {
    let marks: &mut Vec<Mark> = match inline {
        Inline::Text { marks, .. }
        | Inline::HardBreak { marks }
        | Inline::Mention { marks, .. }
        | Inline::Citation { marks, .. } => marks,
    };
    marks.sort_by_key(Mark::rank);
}
