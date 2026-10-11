//! [`Draft`] → TipTap document (`deino_doc::Doc`).
//!
//! Each section is a level-3 heading. Consecutive bullets form one bullet
//! list and consecutive to-dos one checklist; a block's details are a list
//! nested under it. Every run carries the `origin` mark: the user's lines
//! keep their own marks under `user`, everything else is `ai`. A block's
//! evidence follows its text as `citation` nodes (also `ai`), after one
//! space so Readable Markdown reads `text [4:31]`.

use deino_doc::{
    Block, CitationAttrs, Doc, HeadingAttrs, Inline, ListItem, Mark, OriginAttrs, TaskItem,
    TaskItemAttrs,
};

use super::Evidence;
use super::assemble::{Body, Draft, DraftBlock, DraftDetail, DraftKind, DraftSection};

/// Section headings are level 3, Granola's small bold section heads.
const SECTION_LEVEL: u8 = 3;

/// The document of `draft`; the empty document when it has no sections.
pub fn to_doc(draft: &Draft) -> Doc {
    let content: Vec<Block> = draft.sections.iter().flat_map(section_blocks).collect();
    if content.is_empty() {
        return Doc::empty();
    }
    Doc { content }.normalized()
}

fn section_blocks(section: &DraftSection) -> Vec<Block> {
    let origin = if section.heading_is_users {
        OriginAttrs::USER
    } else {
        OriginAttrs::AI
    };
    let mut blocks = vec![Block::Heading {
        attrs: HeadingAttrs {
            level: SECTION_LEVEL,
        },
        content: vec![Inline::text(section.heading.clone(), vec![mark(origin)])],
    }];
    for run in runs(&section.blocks) {
        blocks.extend(run_blocks(run));
    }
    blocks
}

/// The blocks split into runs of the same list kind (paragraphs alone).
fn runs(blocks: &[DraftBlock]) -> Vec<&[DraftBlock]> {
    let mut runs = Vec::new();
    let mut start = 0;
    for end in 1..=blocks.len() {
        let next = blocks.get(end).map(|block| list_kind(block.kind));
        let current = blocks.get(start).map(|block| list_kind(block.kind));
        if next != current || current == Some(ListKind::None) {
            if let Some(run) = blocks.get(start..end) {
                runs.push(run);
            }
            start = end;
        }
    }
    runs
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Bullets,
    Tasks,
    None,
}

fn list_kind(kind: DraftKind) -> ListKind {
    match kind {
        DraftKind::Bullet => ListKind::Bullets,
        DraftKind::Todo { .. } => ListKind::Tasks,
        DraftKind::Paragraph => ListKind::None,
    }
}

fn run_blocks(run: &[DraftBlock]) -> Vec<Block> {
    let Some(first) = run.first() else {
        return Vec::new();
    };
    match first.kind {
        DraftKind::Bullet => vec![Block::BulletList {
            content: run
                .iter()
                .map(|block| ListItem {
                    content: item_blocks(block),
                })
                .collect(),
        }],
        DraftKind::Todo { .. } => vec![Block::TaskList {
            content: run.iter().map(task_item).collect(),
        }],
        DraftKind::Paragraph => run.iter().flat_map(item_blocks).collect(),
    }
}

fn task_item(block: &DraftBlock) -> TaskItem {
    let checked = matches!(block.kind, DraftKind::Todo { checked: true });
    TaskItem {
        attrs: TaskItemAttrs { checked },
        content: item_blocks(block),
    }
}

/// The block's paragraph, then its details as a nested bullet list.
fn item_blocks(block: &DraftBlock) -> Vec<Block> {
    let mut content = body_inlines(&block.body);
    content.extend(citations(&block.evidence));
    let mut blocks = vec![Block::paragraph(content)];
    if !block.details.is_empty() {
        blocks.push(Block::BulletList {
            content: block.details.iter().map(detail_item).collect(),
        });
    }
    blocks
}

fn detail_item(detail: &DraftDetail) -> ListItem {
    let mut content = vec![Inline::text(
        detail.text.clone(),
        vec![mark(OriginAttrs::AI)],
    )];
    content.extend(citations(&detail.evidence));
    ListItem {
        content: vec![Block::paragraph(content)],
    }
}

fn body_inlines(body: &Body) -> Vec<Inline> {
    match body {
        Body::Ai(text) if text.is_empty() => Vec::new(),
        Body::Ai(text) => vec![Inline::text(text.clone(), vec![mark(OriginAttrs::AI)])],
        Body::User(content) => content.iter().cloned().map(as_users).collect(),
    }
}

/// `inline` with the user's origin as its outermost mark.
fn as_users(mut inline: Inline) -> Inline {
    let marks = match &mut inline {
        Inline::Text { marks, .. }
        | Inline::HardBreak { marks }
        | Inline::Mention { marks, .. }
        | Inline::Citation { marks, .. } => marks,
    };
    marks.retain(|mark| !matches!(mark, Mark::Origin { .. }));
    marks.insert(0, mark(OriginAttrs::USER));
    inline
}

/// A space, then one citation per evidence; nothing without evidence.
fn citations(evidence: &[Evidence]) -> Vec<Inline> {
    if evidence.is_empty() {
        return Vec::new();
    }
    let ai = || vec![mark(OriginAttrs::AI)];
    let mut inlines = vec![Inline::text(" ", ai())];
    inlines.extend(evidence.iter().map(|evidence| Inline::Citation {
        attrs: CitationAttrs {
            segment_id: evidence.segment_id,
            start_ms: evidence.start,
        },
        marks: ai(),
    }));
    inlines
}

fn mark(attrs: OriginAttrs) -> Mark {
    Mark::Origin { attrs }
}

#[cfg(test)]
mod convert_tests;
