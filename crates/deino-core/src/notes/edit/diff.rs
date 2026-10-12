//! The preview: which blocks an edit changed, added or removed, and within
//! a changed block which words.
//!
//! The two documents' text blocks are aligned by kind and text (longest
//! common subsequence). Between two kept blocks, removed and added blocks
//! pair up in order as changed blocks (a word diff); the rest are removed
//! or added whole. Unchanged blocks are left out.

use deino_doc::Doc;

use super::blocks::{TextBlock, plain_text, text_blocks};
use super::model::{BlockChange, DiffBlock, DiffSegment, SegmentChange, TextKind};
use super::words::{common, word_diff};

/// The changed blocks from `before` to `after`, in reading order.
pub fn diff(before: &Doc, after: &Doc) -> Vec<DiffBlock> {
    let old = text_blocks(before);
    let new = text_blocks(after);
    let old_keys: Vec<(TextKind, String)> = old.iter().map(key).collect();
    let new_keys: Vec<(TextKind, String)> = new.iter().map(key).collect();
    let mut rows = Vec::new();
    let (mut i, mut j) = (0, 0);
    let ends = std::iter::once((old.len(), new.len()));
    for (kept_i, kept_j) in common(&old_keys, &new_keys).into_iter().chain(ends) {
        let removed = old.get(i..kept_i).unwrap_or_default();
        let added = new.get(j..kept_j).unwrap_or_default();
        rows.extend(between(removed, added));
        (i, j) = (kept_i + 1, kept_j + 1);
    }
    rows
}

fn key(block: &TextBlock) -> (TextKind, String) {
    (block.kind, plain_text(&block.content))
}

/// The rows of a gap between two kept blocks: same-kind pairs are changed
/// blocks, the rest removed or added whole.
fn between(removed: &[TextBlock], added: &[TextBlock]) -> Vec<DiffBlock> {
    let paired = removed.len().min(added.len());
    let mut rows = Vec::new();
    for (old, new) in removed.iter().zip(added) {
        if old.kind == new.kind {
            rows.push(DiffBlock {
                change: BlockChange::Changed,
                kind: new.kind,
                section: new.section.clone(),
                segments: word_diff(&plain_text(&old.content), &plain_text(&new.content)),
            });
        } else {
            rows.push(whole(old, BlockChange::Removed, SegmentChange::Removed));
            rows.push(whole(new, BlockChange::Added, SegmentChange::Added));
        }
    }
    let gone = removed.iter().skip(paired);
    rows.extend(gone.map(|old| whole(old, BlockChange::Removed, SegmentChange::Removed)));
    let new = added.iter().skip(paired);
    rows.extend(new.map(|new| whole(new, BlockChange::Added, SegmentChange::Added)));
    rows.retain(has_text);
    rows
}

fn whole(block: &TextBlock, change: BlockChange, segment: SegmentChange) -> DiffBlock {
    DiffBlock {
        change,
        kind: block.kind,
        section: block.section.clone(),
        segments: vec![DiffSegment {
            change: segment,
            text: plain_text(&block.content),
        }],
    }
}

/// Blank blocks (an empty paragraph) are not worth a row.
fn has_text(row: &DiffBlock) -> bool {
    row.segments
        .iter()
        .any(|segment| !segment.text.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::edit::test_support::{ai, bullets, doc, heading, para, user};

    fn notes(owner: &str) -> Doc {
        doc(vec![
            heading("Pricing"),
            bullets(vec![vec![user(&format!("{owner} owns pricing"))]]),
            para(vec![ai("Ship on Friday.")]),
        ])
    }

    #[test]
    fn a_changed_line_is_one_row_with_its_words() {
        let rows = diff(&notes("Niall"), &notes("Nyal"));

        assert_eq!(rows.len(), 1);
        let row = rows.first().unwrap();
        assert_eq!(row.change, BlockChange::Changed);
        assert_eq!(row.section.as_deref(), Some("Pricing"));
        assert_eq!(
            row.segments.first().map(|segment| segment.text.as_str()),
            Some("Niall")
        );
    }

    #[test]
    fn removed_and_added_blocks_are_whole_rows() {
        let after = doc(vec![
            heading("Pricing"),
            bullets(vec![vec![user("Niall owns pricing")]]),
            heading("Risks"),
        ]);

        let rows = diff(&notes("Niall"), &after);
        let changes: Vec<BlockChange> = rows.iter().map(|row| row.change).collect();

        assert_eq!(changes, [BlockChange::Removed, BlockChange::Added]);
        assert_eq!(diff(&after, &after), []);
    }
}
