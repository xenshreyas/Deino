//! From the model's answer to notes that keep the user's word: citations
//! resolved, provenance checked, every user line present.
//!
//! The rules, in order (PARITY_PLAN NOT-02, Granola §3.2):
//!
//! 1. a block the model marks "user" must be one of the user's lines; it
//!    is rendered from the user's own document, else it becomes AI text;
//! 2. an "ai" block that is exactly a user line is the user's;
//! 3. every evidence reference must name a transcript line (`citations`);
//! 4. a user line the model left out is put back in the section it shares
//!    the most words with, else in a closing "From your notes" section.

use std::sync::LazyLock;

use deino_doc::Inline;
use regex::Regex;

use super::Evidence;
use super::citations::SegmentIndex;
use super::user_lines::{LineKind, LineMatcher, UserLine, comparable, word_overlap};
use super::wire::{BlockKind, EnhancedBlock, EnhancedDoc, EnhancedSection, Origin};

/// The section for user lines that fit nowhere else.
pub const LEFTOVER_HEADING: &str = "From your notes";
/// The least word overlap that places a dropped user line in a section.
const PLACEMENT_OVERLAP: f64 = 0.08;

/// References the model sometimes writes into the text itself.
static INLINE_REFERENCES: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\s*\[\s*[sS]\d+(?:\s*[,;]\s*[sS]\d+)*\s*\]").ok());

/// The notes, checked and resolved, ready to become a document.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Draft {
    /// The sections, in order; none is empty.
    pub sections: Vec<DraftSection>,
}

/// One section.
#[derive(Clone, Debug, PartialEq)]
pub struct DraftSection {
    /// Its heading.
    pub heading: String,
    /// True when the user typed this heading in their notes.
    pub heading_is_users: bool,
    /// Its blocks.
    pub blocks: Vec<DraftBlock>,
}

/// One block.
#[derive(Clone, Debug, PartialEq)]
pub struct DraftBlock {
    /// How it reads.
    pub kind: DraftKind,
    /// Its text, and who wrote it.
    pub body: Body,
    /// The transcript moments behind it.
    pub evidence: Vec<Evidence>,
    /// AI sub-points.
    pub details: Vec<DraftDetail>,
}

/// How a block reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftKind {
    /// A bullet point.
    Bullet,
    /// A paragraph.
    Paragraph,
    /// A checklist item.
    Todo {
        /// Ticked by the user.
        checked: bool,
    },
}

/// Who wrote a block, and its content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// AI text.
    Ai(String),
    /// A user line, as they typed it.
    User(Vec<Inline>),
}

/// An AI sub-point.
#[derive(Clone, Debug, PartialEq)]
pub struct DraftDetail {
    /// Its text.
    pub text: String,
    /// The transcript moments behind it.
    pub evidence: Vec<Evidence>,
}

/// What the answer is checked against.
#[derive(Clone, Copy, Debug)]
pub struct Checks<'a> {
    /// The lines the user typed.
    pub lines: &'a [UserLine],
    /// The headings the user typed.
    pub headings: &'a [String],
    /// The transcript's references.
    pub index: &'a SegmentIndex,
}

/// Checks and resolves the model's `answer` (see the module docs).
pub fn assemble(answer: &EnhancedDoc, checks: Checks<'_>) -> Draft {
    let mut matcher = LineMatcher::new(checks.lines);
    let mut sections: Vec<DraftSection> = answer
        .sections
        .iter()
        .map(|section| draft_section(section, checks, &mut matcher))
        .collect();
    place_dropped_lines(&mut sections, checks.lines, &matcher.unclaimed());
    sections.retain(|section| !section.blocks.is_empty());
    Draft { sections }
}

fn draft_section(
    section: &EnhancedSection,
    checks: Checks<'_>,
    matcher: &mut LineMatcher,
) -> DraftSection {
    let heading = clean_text(&section.heading);
    let key = comparable(&heading);
    DraftSection {
        heading_is_users: checks.headings.iter().any(|typed| comparable(typed) == key),
        heading,
        blocks: section
            .blocks
            .iter()
            .filter_map(|block| draft_block(block, checks, matcher))
            .collect(),
    }
}

fn draft_block(
    block: &EnhancedBlock,
    checks: Checks<'_>,
    matcher: &mut LineMatcher,
) -> Option<DraftBlock> {
    let text = clean_text(&block.text);
    let details = drafted_details(block, checks.index);
    if text.is_empty() && details.is_empty() {
        return None;
    }
    let claimed = match block.origin {
        Origin::User => matcher.claim(&text),
        Origin::Ai => matcher.claim_exact(&text),
    };
    let user = claimed.and_then(|at| checks.lines.get(at));
    Some(DraftBlock {
        kind: kind_of(block.kind, user.map(|line| line.kind)),
        body: user.map_or(Body::Ai(text), |line| Body::User(line.content.clone())),
        evidence: checks.index.resolve(&block.evidence),
        details,
    })
}

fn drafted_details(block: &EnhancedBlock, index: &SegmentIndex) -> Vec<DraftDetail> {
    block
        .details
        .iter()
        .map(|detail| DraftDetail {
            text: clean_text(&detail.text),
            evidence: index.resolve(&detail.evidence),
        })
        .filter(|detail| !detail.text.is_empty())
        .collect()
}

/// A user's checkbox stays a checkbox, with their tick; otherwise the
/// model's choice.
fn kind_of(kind: BlockKind, user: Option<LineKind>) -> DraftKind {
    match (user, kind) {
        (Some(LineKind::Todo { checked }), _) => DraftKind::Todo { checked },
        (_, BlockKind::Bullet) => DraftKind::Bullet,
        (_, BlockKind::Paragraph) => DraftKind::Paragraph,
        (_, BlockKind::Todo) => DraftKind::Todo { checked: false },
    }
}

/// Puts back the user lines the model left out (rule 4).
fn place_dropped_lines(sections: &mut Vec<DraftSection>, lines: &[UserLine], dropped: &[usize]) {
    let mut leftovers = Vec::new();
    for line in dropped.iter().filter_map(|&at| lines.get(at)) {
        let block = DraftBlock {
            kind: kind_of(BlockKind::Bullet, Some(line.kind)),
            body: Body::User(line.content.clone()),
            evidence: Vec::new(),
            details: Vec::new(),
        };
        match best_section(sections, &line.text) {
            Some(section) => section.blocks.push(block),
            None => leftovers.push(block),
        }
    }
    if !leftovers.is_empty() {
        sections.push(DraftSection {
            heading: LEFTOVER_HEADING.to_owned(),
            heading_is_users: false,
            blocks: leftovers,
        });
    }
}

/// The section whose words `text` shares the most of, if any shares enough.
fn best_section<'a>(sections: &'a mut [DraftSection], text: &str) -> Option<&'a mut DraftSection> {
    sections
        .iter_mut()
        .map(|section| {
            let score = word_overlap(text, &section_words(section));
            (section, score)
        })
        .filter(|(_, score)| *score >= PLACEMENT_OVERLAP)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(section, _)| section)
}

/// A section's heading and AI text, the words a user line is placed by.
fn section_words(section: &DraftSection) -> String {
    let mut words = section.heading.clone();
    for block in &section.blocks {
        if let Body::Ai(text) = &block.body {
            words.push(' ');
            words.push_str(text);
        }
        for detail in &block.details {
            words.push(' ');
            words.push_str(&detail.text);
        }
    }
    words
}

/// The model's text without list markers, inline references or stray
/// Markdown emphasis.
fn clean_text(text: &str) -> String {
    let mut text = text.trim();
    for marker in ["- [ ] ", "[ ] ", "- ", "* ", "• "] {
        if let Some(rest) = text.strip_prefix(marker) {
            text = rest.trim_start();
            break;
        }
    }
    let text = match INLINE_REFERENCES.as_ref() {
        Some(pattern) => pattern.replace_all(text, "").into_owned(),
        None => text.to_owned(),
    };
    text.replace("**", "").trim().to_owned()
}

#[cfg(test)]
mod assemble_tests;
