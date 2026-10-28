//! What the writer falls back on when an answer cannot be used (pure).

use deino_llm::LlmError;

use crate::enhance::transcript::TranscriptLine;
use crate::enhance::wire::{EnhancedDoc, EnhancedSection, Origin};

/// Whether `error` is the answer's fault, not the request's: an answer cut
/// off at the token cap, or one that did not parse even after the repair.
/// Asking again, or for less, can work; for any other error it cannot.
pub(super) fn is_answer_failure(error: &LlmError) -> bool {
    matches!(error, LlmError::Truncated | LlmError::InvalidOutput { .. })
}

/// `lines` in two halves, or none when there is too little to split.
pub(super) fn halve(lines: &[TranscriptLine]) -> Vec<&[TranscriptLine]> {
    if lines.len() < 2 {
        return Vec::new();
    }
    let (first, second) = lines.split_at(lines.len() / 2);
    vec![first, second]
}

/// The merge's answer, or the parts merged here when the merge's answer
/// could not be used.
pub(super) fn or_merge_locally(
    merged: Result<EnhancedDoc, LlmError>,
    noted: &[(&[TranscriptLine], EnhancedDoc)],
) -> Result<EnhancedDoc, LlmError> {
    match merged {
        Err(error) if is_answer_failure(&error) => {
            log::warn!(target: "enhance", "the merge failed ({:?}); merging {} parts here", error.kind(), noted.len());
            Ok(merge_locally(noted))
        }
        other => other,
    }
}

/// The parts' notes as one: sections in the order they first appear, a
/// heading's blocks from every part in part order, and a line the user
/// typed kept once (each part may copy it). The references are global, so
/// the evidence stays valid.
pub(super) fn merge_locally(noted: &[(&[TranscriptLine], EnhancedDoc)]) -> EnhancedDoc {
    let mut sections: Vec<EnhancedSection> = Vec::new();
    for section in noted.iter().flat_map(|(_, notes)| &notes.sections) {
        add_section(&mut sections, section);
    }
    EnhancedDoc { sections }
}

/// `section`'s blocks under the section of the same heading, or as a new one.
fn add_section(sections: &mut Vec<EnhancedSection>, section: &EnhancedSection) {
    let key = heading_key(&section.heading);
    if let Some(seen) = sections
        .iter_mut()
        .find(|seen| heading_key(&seen.heading) == key)
    {
        append_blocks(seen, section);
        return;
    }
    let mut new = EnhancedSection {
        heading: section.heading.clone(),
        blocks: Vec::new(),
    };
    append_blocks(&mut new, section);
    sections.push(new);
}

/// `from`'s blocks after `into`'s, skipping a user line `into` already has.
fn append_blocks(into: &mut EnhancedSection, from: &EnhancedSection) {
    for block in &from.blocks {
        let repeated = block.origin == Origin::User
            && into
                .blocks
                .iter()
                .any(|seen| seen.origin == Origin::User && seen.text == block.text);
        if !repeated {
            into.blocks.push(block.clone());
        }
    }
}

fn heading_key(heading: &str) -> String {
    heading.trim().to_lowercase()
}

#[cfg(test)]
mod fallback_tests;
