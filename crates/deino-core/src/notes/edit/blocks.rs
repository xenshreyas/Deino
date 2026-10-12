//! The document as text blocks: every paragraph, heading and code block in
//! reading order, numbered from 0. The UI numbers its selection the same way
//! (ProseMirror's textblocks in document order), and the prompt names them
//! `b1`, `b2`, … from these positions.

use deino_doc::{Author, Block, Doc, Inline, Mark};

use super::model::TextKind;

/// One paragraph, heading or code block of the document.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBlock {
    /// How it reads.
    pub kind: TextKind,
    /// Its inline content.
    pub content: Vec<Inline>,
    /// The text of the heading it sits under (itself, for a heading).
    pub section: Option<String>,
}

/// Every text block of `doc`, in reading order.
pub fn text_blocks(doc: &Doc) -> Vec<TextBlock> {
    let mut walk = Collect::default();
    walk.blocks(&doc.content, None);
    walk.found
}

/// Calls `visit` with the position and content of every text block, in
/// reading order, letting it change the content.
pub fn for_each_text_block(doc: &mut Doc, visit: &mut impl FnMut(usize, &mut Vec<Inline>)) {
    let mut next = 0;
    visit_blocks(&mut doc.content, &mut next, visit);
}

fn visit_blocks(
    blocks: &mut [Block],
    next: &mut usize,
    visit: &mut impl FnMut(usize, &mut Vec<Inline>),
) {
    for block in blocks {
        match block {
            Block::Paragraph { content }
            | Block::Heading { content, .. }
            | Block::CodeBlock { content, .. } => {
                visit(*next, content);
                *next += 1;
            }
            Block::BulletList { content } | Block::OrderedList { content, .. } => {
                for item in content {
                    visit_blocks(&mut item.content, next, visit);
                }
            }
            Block::TaskList { content } => {
                for item in content {
                    visit_blocks(&mut item.content, next, visit);
                }
            }
            Block::Blockquote { content } => visit_blocks(content, next, visit),
            Block::HorizontalRule | Block::Image { .. } => {}
        }
    }
}

/// The block's text as a person reads it: mentions as `@Name`, line breaks
/// as spaces, citations left out.
pub fn plain_text(content: &[Inline]) -> String {
    content
        .iter()
        .map(|inline| match inline {
            Inline::Text { text, .. } => text.clone(),
            Inline::HardBreak { .. } => " ".to_owned(),
            Inline::Mention { attrs, .. } => format!("@{}", attrs.label),
            Inline::Citation { .. } => String::new(),
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Who wrote the block's text: `user`, `ai`, or `mixed`.
pub fn origin_label(content: &[Inline]) -> &'static str {
    let mut user = false;
    let mut ai = false;
    for inline in content {
        if let Inline::Citation { .. } = inline {
            continue;
        }
        match author(inline.marks()) {
            Some(Author::User) | None => user = true,
            Some(Author::Ai) => ai = true,
        }
    }
    match (user, ai) {
        (true, true) => "mixed",
        (false, true) => "ai",
        _ => "user",
    }
}

fn author(marks: &[Mark]) -> Option<Author> {
    marks.iter().find_map(|mark| match mark {
        Mark::Origin { attrs } => Some(attrs.author),
        _ => None,
    })
}

/// The walk behind [`text_blocks`].
#[derive(Default)]
struct Collect {
    found: Vec<TextBlock>,
    section: Option<String>,
}

impl Collect {
    /// `blocks`; `first` is the kind of a list item's first paragraph.
    fn blocks(&mut self, blocks: &[Block], first: Option<TextKind>) {
        for (index, block) in blocks.iter().enumerate() {
            let lead = if index == 0 { first } else { None };
            self.block(block, lead);
        }
    }

    fn block(&mut self, block: &Block, lead: Option<TextKind>) {
        match block {
            Block::Paragraph { content } => {
                self.push(lead.unwrap_or(TextKind::Paragraph), content);
            }
            Block::Heading { content, .. } => {
                self.section = Some(plain_text(content));
                self.push(TextKind::Heading, content);
            }
            Block::CodeBlock { content, .. } => self.push(TextKind::Code, content),
            Block::BulletList { content } => self.items(content, TextKind::Bullet),
            Block::OrderedList { content, .. } => self.items(content, TextKind::Numbered),
            Block::TaskList { content } => {
                for item in content {
                    self.blocks(&item.content, Some(TextKind::Todo));
                }
            }
            Block::Blockquote { content } => {
                for quoted in content {
                    self.block(quoted, Some(TextKind::Quote));
                }
            }
            Block::HorizontalRule | Block::Image { .. } => {}
        }
    }

    fn items(&mut self, items: &[deino_doc::ListItem], kind: TextKind) {
        for item in items {
            self.blocks(&item.content, Some(kind));
        }
    }

    fn push(&mut self, kind: TextKind, content: &[Inline]) {
        self.found.push(TextBlock {
            kind,
            content: content.to_vec(),
            section: self.section.clone(),
        });
    }
}
