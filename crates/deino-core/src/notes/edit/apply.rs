//! A [`Plan`] of block changes applied to the document in one pass.
//!
//! Positions are the original text blocks' ([`super::blocks`]), so every
//! change is planned against the document the model saw. Deleting a list
//! item's first line deletes the item with what is nested in it; a block
//! inserted after a list item's line becomes a new item of that list;
//! lists, items and quotes left empty disappear.

use std::collections::{BTreeMap, BTreeSet};

use deino_doc::{Block, Doc, HeadingAttrs, Inline, ListItem, TaskItem, TaskItemAttrs};

use super::wire::NewKind;

/// New section headings match the notes' (level 3).
const HEADING_LEVEL: u8 = 3;

/// A block to insert.
#[derive(Clone, Debug, PartialEq)]
pub struct NewBlock {
    /// Its kind (a list item takes its list's kind instead).
    pub kind: NewKind,
    /// Its content.
    pub content: Vec<Inline>,
}

/// The changes to make, by text block position.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    /// New content for these blocks.
    pub replace: BTreeMap<usize, Vec<Inline>>,
    /// These blocks go.
    pub delete: BTreeSet<usize>,
    /// New blocks after these ones, in order.
    pub insert: BTreeMap<usize, Vec<NewBlock>>,
}

impl Plan {
    /// True when the plan changes nothing.
    pub fn is_empty(&self) -> bool {
        self.replace.is_empty() && self.delete.is_empty() && self.insert.is_empty()
    }
}

/// `doc` with `plan` applied.
pub fn apply(doc: &Doc, plan: &Plan) -> Doc {
    let mut walk = Walk { plan, next: 0 };
    let content = walk.blocks(&doc.content);
    if content.is_empty() {
        return Doc::empty();
    }
    Doc { content }.normalized()
}

struct Walk<'a> {
    plan: &'a Plan,
    next: usize,
}

impl Walk<'_> {
    fn blocks(&mut self, blocks: &[Block]) -> Vec<Block> {
        let mut out = Vec::new();
        for block in blocks {
            self.block(block, &mut out);
        }
        out
    }

    fn block(&mut self, block: &Block, out: &mut Vec<Block>) {
        match block {
            Block::Paragraph { .. } | Block::Heading { .. } | Block::CodeBlock { .. } => {
                let position = self.take();
                out.extend(self.kept(block, position));
                let inserted = self.inserted(position);
                out.extend(inserted.iter().map(standalone));
            }
            Block::BulletList { content } => {
                let items = self.list_items(content);
                if !items.is_empty() {
                    out.push(Block::BulletList { content: items });
                }
            }
            Block::OrderedList { attrs, content } => {
                let items = self.list_items(content);
                if !items.is_empty() {
                    out.push(Block::OrderedList {
                        attrs: *attrs,
                        content: items,
                    });
                }
            }
            Block::TaskList { content } => self.task_list(content, out),
            Block::Blockquote { content } => {
                let quoted = self.blocks(content);
                if !quoted.is_empty() {
                    out.push(Block::Blockquote { content: quoted });
                }
            }
            Block::HorizontalRule | Block::Image { .. } => out.push(block.clone()),
        }
    }

    fn task_list(&mut self, items: &[TaskItem], out: &mut Vec<Block>) {
        let mut kept = Vec::new();
        for item in items {
            let (content, inserted) = self.item(&item.content);
            kept.extend(content.map(|content| TaskItem {
                attrs: item.attrs,
                content,
            }));
            kept.extend(inserted.into_iter().map(|block| TaskItem {
                attrs: TaskItemAttrs { checked: false },
                content: vec![Block::paragraph(block.content)],
            }));
        }
        if !kept.is_empty() {
            out.push(Block::TaskList { content: kept });
        }
    }

    fn list_items(&mut self, items: &[ListItem]) -> Vec<ListItem> {
        let mut kept = Vec::new();
        for item in items {
            let (content, inserted) = self.item(&item.content);
            kept.extend(content.map(|content| ListItem { content }));
            kept.extend(inserted.into_iter().map(|block| ListItem {
                content: vec![Block::paragraph(block.content)],
            }));
        }
        kept
    }

    /// An item's new content (`None` when its line was deleted) and the
    /// items to insert after it.
    fn item(&mut self, content: &[Block]) -> (Option<Vec<Block>>, Vec<NewBlock>) {
        let Some((first @ Block::Paragraph { .. }, rest)) = content.split_first() else {
            return (Some(self.blocks(content)), Vec::new());
        };
        let position = self.take();
        let line = self.kept(first, position);
        let nested = self.blocks(rest);
        let inserted = self.inserted(position).to_vec();
        let content = line.map(|line| std::iter::once(line).chain(nested).collect());
        (content, inserted)
    }

    /// The next text block's position.
    fn take(&mut self) -> usize {
        let position = self.next;
        self.next += 1;
        position
    }

    /// The block at `position`, replaced as planned; `None` once deleted.
    fn kept(&self, block: &Block, position: usize) -> Option<Block> {
        if self.plan.delete.contains(&position) {
            return None;
        }
        let Some(content) = self.plan.replace.get(&position) else {
            return Some(block.clone());
        };
        let content = content.clone();
        Some(match block {
            Block::Heading { attrs, .. } => Block::Heading {
                attrs: *attrs,
                content,
            },
            _ => Block::Paragraph { content },
        })
    }

    fn inserted(&self, position: usize) -> &[NewBlock] {
        self.plan.insert.get(&position).map_or(&[], Vec::as_slice)
    }
}

/// A new block outside a list.
fn standalone(block: &NewBlock) -> Block {
    let paragraph = || Block::paragraph(block.content.clone());
    match block.kind {
        NewKind::Heading => Block::Heading {
            attrs: HeadingAttrs {
                level: HEADING_LEVEL,
            },
            content: block.content.clone(),
        },
        NewKind::Paragraph => paragraph(),
        NewKind::Bullet => Block::BulletList {
            content: vec![ListItem {
                content: vec![paragraph()],
            }],
        },
        NewKind::Todo => Block::TaskList {
            content: vec![TaskItem {
                attrs: TaskItemAttrs { checked: false },
                content: vec![paragraph()],
            }],
        },
    }
}

#[cfg(test)]
mod apply_tests;
