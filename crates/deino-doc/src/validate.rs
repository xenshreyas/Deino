//! The schema rules the types cannot express, checked before a document is
//! stored.
//!
//! Adapted from anarlog's `crates/tiptap/src/validate.rs` (MIT): the same
//! ProseMirror content expressions (`doc: block+`, `listItem: paragraph
//! block*`, non-empty lists and quotes), checked over the typed model instead
//! of raw JSON, plus Deino's own rules for marks, links and the NOT-02
//! provenance and citation nodes.

use std::fmt;

use crate::{Author, Block, Doc, DocError, Inline, Mark};

/// One broken rule and where it is, as a JSON-ish path (`content[2].content[0]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    /// Where in the document.
    pub path: String,
    /// What is wrong; never quotes the document's text.
    pub message: &'static str,
}

impl fmt::Display for Issue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "at {}: {}", self.path, self.message)
    }
}

/// URL schemes a link may use; anything else (`javascript:`, `file:`) is refused.
const SAFE_SCHEMES: [&str; 3] = ["https://", "http://", "mailto:"];

/// Heading levels the schema allows.
const HEADING_LEVELS: std::ops::RangeInclusive<u8> = 1..=6;

/// Checks every rule; fails with all the issues found.
pub fn validate(doc: &Doc) -> Result<(), DocError> {
    let mut issues = Vec::new();
    if doc.content.is_empty() {
        issues.push(issue("content", "a document needs at least one block"));
    }
    blocks(&doc.content, "content", &mut issues);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(DocError::Invalid(issues))
    }
}

/// True for links the app may open: `http`, `https` and `mailto`.
pub fn is_safe_href(href: &str) -> bool {
    let lower = href.trim_start().to_ascii_lowercase();
    SAFE_SCHEMES.iter().any(|scheme| lower.starts_with(scheme))
}

fn issue(path: &str, message: &'static str) -> Issue {
    Issue {
        path: path.to_owned(),
        message,
    }
}

fn blocks(content: &[Block], path: &str, issues: &mut Vec<Issue>) {
    for (index, block) in content.iter().enumerate() {
        check_block(block, &format!("{path}[{index}]"), issues);
    }
}

fn check_block(block: &Block, path: &str, issues: &mut Vec<Issue>) {
    match block {
        Block::Heading { attrs, .. } if !HEADING_LEVELS.contains(&attrs.level) => {
            issues.push(issue(path, "a heading's level must be 1 to 6"));
        }
        Block::BulletList { content } | Block::OrderedList { content, .. } => {
            let items: Vec<&[Block]> = content.iter().map(|item| item.content.as_slice()).collect();
            list(&items, path, issues);
        }
        Block::TaskList { content } => {
            let items: Vec<&[Block]> = content.iter().map(|item| item.content.as_slice()).collect();
            list(&items, path, issues);
        }
        Block::Blockquote { content } if content.is_empty() => {
            issues.push(issue(path, "a quote needs at least one block"));
        }
        Block::Blockquote { content } => blocks(content, &format!("{path}.content"), issues),
        Block::CodeBlock { content, .. } if !content.iter().all(is_plain_text) => {
            issues.push(issue(path, "a code block holds unformatted text only"));
        }
        Block::Image { attrs } if attrs.src.trim().is_empty() => {
            issues.push(issue(path, "an image needs a source"));
        }
        _ => {}
    }
    inlines(block.inline_content(), &format!("{path}.content"), issues);
}

/// Lists are non-empty, and each item starts with a paragraph.
fn list(items: &[&[Block]], path: &str, issues: &mut Vec<Issue>) {
    if items.is_empty() {
        issues.push(issue(path, "a list needs at least one item"));
    }
    for (index, content) in items.iter().enumerate() {
        let item_path = format!("{path}.content[{index}]");
        if !matches!(content.first(), Some(Block::Paragraph { .. })) {
            issues.push(issue(&item_path, "a list item must start with a paragraph"));
        }
        blocks(content, &format!("{item_path}.content"), issues);
    }
}

fn is_plain_text(inline: &Inline) -> bool {
    matches!(inline, Inline::Text { marks, .. } if marks.is_empty())
}

fn inlines(content: &[Inline], path: &str, issues: &mut Vec<Issue>) {
    for (index, inline) in content.iter().enumerate() {
        let path = format!("{path}[{index}]");
        check_inline(inline, &path, issues);
        marks(inline.marks(), &path, issues);
    }
}

fn check_inline(inline: &Inline, path: &str, issues: &mut Vec<Issue>) {
    match inline {
        Inline::Text { text, .. } if text.is_empty() => {
            issues.push(issue(path, "a text node cannot be empty"));
        }
        Inline::Mention { attrs, .. } if attrs.label.trim().is_empty() => {
            issues.push(issue(path, "a mention needs a label"));
        }
        _ => {}
    }
}

/// One mark per kind; code combines with provenance only; links are safe;
/// only AI text can be "edited".
fn marks(marks: &[Mark], path: &str, issues: &mut Vec<Issue>) {
    for (index, mark) in marks.iter().enumerate() {
        if marks
            .iter()
            .skip(index + 1)
            .any(|other| other.same_kind(mark))
        {
            issues.push(issue(path, "a mark appears twice"));
        }
        match mark {
            Mark::Link { attrs } if !is_safe_href(&attrs.href) => {
                issues.push(issue(path, "a link must be http, https or mailto"));
            }
            Mark::Origin { attrs } if attrs.edited && attrs.author == Author::User => {
                issues.push(issue(path, "only AI text can be marked edited"));
            }
            Mark::Code if marks.iter().any(is_formatting) => {
                issues.push(issue(path, "code combines with no formatting mark"));
            }
            _ => {}
        }
    }
}

/// Every mark except code itself and provenance.
fn is_formatting(mark: &Mark) -> bool {
    !matches!(mark, Mark::Code | Mark::Origin { .. })
}
