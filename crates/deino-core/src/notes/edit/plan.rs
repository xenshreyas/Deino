//! From the notes to the prompt's blocks, and from the model's answer to a
//! [`Plan`] that keeps to what the model may change.
//!
//! The rules hold whatever the model answers: a change to a block outside
//! the selection is dropped; a locked block (outside the selection, a code
//! block, or one whose text had a secret redacted) may be deleted but its
//! text never replaced, so a secret is neither sent nor overwritten with
//! `[redacted]`; unknown block IDs are dropped.

use super::apply::{NewBlock, Plan};
use super::blocks::{TextBlock, origin_label};
use super::model::TextKind;
use super::prompt::PromptBlock;
use super::rewrite::{Refs, rewrite, shown_text};
use super::scope::{Scope, parse_block_id};
use super::wire::{EditAction, EditAnswer, EditChange};

/// The blocks as the prompt shows them, locked where they may not be
/// rewritten.
pub fn prompt_blocks(blocks: &[TextBlock], refs: &Refs, scope: &Scope) -> Vec<PromptBlock> {
    blocks
        .iter()
        .enumerate()
        .map(|(position, block)| {
            let text = shown_text(&block.content, refs);
            let mut shown =
                PromptBlock::new(position, block.kind, origin_label(&block.content), text);
            shown.locked = !scope.contains(position) || block.kind == TextKind::Code;
            shown
        })
        .collect()
}

/// Puts the redacted texts in place, locking every block that changed.
pub fn redact_blocks(blocks: &mut [PromptBlock], redacted: Vec<String>) {
    for (block, text) in blocks.iter_mut().zip(redacted) {
        if text != block.text {
            block.text = text;
            block.locked = true;
        }
    }
}

/// What the plan is checked against.
pub struct Checks<'a> {
    /// The document's text blocks.
    pub blocks: &'a [TextBlock],
    /// The blocks as shown, with their locks.
    pub shown: &'a [PromptBlock],
    /// What the user selected.
    pub scope: &'a Scope,
    /// The transcript references.
    pub refs: &'a Refs,
}

/// The plan of `answer`, keeping only the changes the rules allow.
pub fn plan(answer: &EditAnswer, checks: &Checks<'_>) -> Plan {
    let mut plan = Plan::default();
    for change in &answer.changes {
        let Some(position) = parse_block_id(&change.block) else {
            continue;
        };
        let (Some(block), Some(shown)) = (checks.blocks.get(position), checks.shown.get(position))
        else {
            continue;
        };
        if !checks.scope.contains(position) {
            continue;
        }
        match change.action {
            EditAction::Delete => {
                plan.delete.insert(position);
            }
            EditAction::Replace if change.text.trim().is_empty() => {
                plan.delete.insert(position);
            }
            EditAction::Replace if !shown.locked => {
                let content = rewrite(&block.content, &change.text, checks.refs);
                if content != block.content {
                    plan.replace.insert(position, content);
                }
            }
            EditAction::Replace => {}
            EditAction::InsertAfter => insert(&mut plan, position, change, checks.refs),
        }
    }
    plan
}

fn insert(plan: &mut Plan, position: usize, change: &EditChange, refs: &Refs) {
    if change.text.trim().is_empty() {
        return;
    }
    plan.insert.entry(position).or_default().push(NewBlock {
        kind: change.kind.unwrap_or_default(),
        content: rewrite(&[], &change.text, refs),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::edit::blocks::text_blocks;
    use crate::notes::edit::test_support::{ai, bullets, cite, doc, heading, line, user};
    use crate::notes::edit::wire::NewKind;

    fn change(action: EditAction, block: &str, text: &str) -> EditChange {
        EditChange {
            action,
            block: block.into(),
            text: text.into(),
            kind: Some(NewKind::Bullet),
        }
    }

    #[test]
    fn changes_outside_the_selection_or_to_locked_text_are_dropped() {
        let notes = doc(vec![
            heading("Pricing"),
            bullets(vec![
                vec![user("price is 10 "), cite(4)],
                vec![ai("password hunter2")],
            ]),
        ]);
        let blocks = text_blocks(&notes);
        let refs = Refs::new(&[line(4, 4, "Ten dollars.")]);
        let scope = Scope::of(&[1, 2], blocks.len());
        let mut shown = prompt_blocks(&blocks, &refs, &scope);
        redact_blocks(
            &mut shown,
            vec![
                "Pricing".into(),
                "price is 10 [s4]".into(),
                "password [redacted]".into(),
            ],
        );
        let answer = EditAnswer {
            changes: vec![
                change(EditAction::Replace, "b1", "Costs"),
                change(EditAction::Replace, "b2", "Price is $10 [s4]"),
                change(EditAction::Replace, "b3", "password [redacted]"),
                change(EditAction::InsertAfter, "b3", "Annual plan discussed [s4]"),
                change(EditAction::Delete, "b42", ""),
            ],
        };
        let checks = Checks {
            blocks: &blocks,
            shown: &shown,
            scope: &scope,
            refs: &refs,
        };

        let plan = plan(&answer, &checks);

        assert_eq!(plan.replace.keys().copied().collect::<Vec<_>>(), [1]);
        assert!(plan.delete.is_empty());
        assert_eq!(plan.insert.keys().copied().collect::<Vec<_>>(), [2]);
        assert!(shown.first().is_some_and(|block| block.locked));
        assert!(shown.get(2).is_some_and(|block| block.locked));
    }
}
