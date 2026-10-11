//! The lines the user typed, and how the model's copies are matched to them.
//!
//! Granola's rule is that everything the user typed is important and keeps
//! its wording. The prompt asks the model to quote each line, but the notes
//! never rely on the model's copy: a block the model marks as the user's is
//! matched back to one of their lines and rendered from the user's own
//! document (marks, mentions and all), a "user" block that matches no line
//! becomes AI text, and a line the model dropped is put back
//! (`super::assemble`). So the black text in the notes is always exactly
//! what the user typed.

use deino_doc::{Block, Doc, Inline, Mark};

/// The share of words two lines must share to be the same line, when they
/// are not equal: the model fixed a typo or dropped a word.
const FUZZY_MATCH: f64 = 0.75;

/// How a user line reads in their notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// A list item.
    Bullet,
    /// A line of a paragraph.
    Paragraph,
    /// A checklist item, ticked or not.
    Todo {
        /// Whether it is ticked.
        checked: bool,
    },
}

/// One line the user typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserLine {
    /// How it reads.
    pub kind: LineKind,
    /// Its inline content as typed, without provenance marks.
    pub content: Vec<Inline>,
    /// Its plain text (mentions as `@Name`).
    pub text: String,
}

/// Every line of `doc`, in reading order. Headings are structure, not
/// lines; images and rules carry no text.
pub fn user_lines(doc: &Doc) -> Vec<UserLine> {
    let mut lines = Vec::new();
    for block in &doc.content {
        collect(block, LineKind::Paragraph, &mut lines);
    }
    lines
}

/// The plain text of the headings of `doc`, in order.
pub fn user_headings(doc: &Doc) -> Vec<String> {
    doc.content
        .iter()
        .filter(|block| matches!(block, Block::Heading { .. }))
        .map(|block| plain_text(block.inline_content()))
        .filter(|text| !text.trim().is_empty())
        .collect()
}

fn collect(block: &Block, kind: LineKind, lines: &mut Vec<UserLine>) {
    match block {
        Block::Paragraph { content } | Block::CodeBlock { content, .. } => {
            push_lines(content, kind, lines);
        }
        Block::BulletList { content } | Block::OrderedList { content, .. } => {
            for item in content {
                collect_item(&item.content, LineKind::Bullet, lines);
            }
        }
        Block::TaskList { content } => {
            for item in content {
                let kind = LineKind::Todo {
                    checked: item.attrs.checked,
                };
                collect_item(&item.content, kind, lines);
            }
        }
        Block::Blockquote { content } => {
            for child in content {
                collect(child, kind, lines);
            }
        }
        Block::Heading { .. } | Block::HorizontalRule | Block::Image { .. } => {}
    }
}

/// An item's first paragraph is the item's line; nested blocks follow.
fn collect_item(content: &[Block], kind: LineKind, lines: &mut Vec<UserLine>) {
    for (index, block) in content.iter().enumerate() {
        let block_kind = if index == 0 { kind } else { LineKind::Bullet };
        collect(block, block_kind, lines);
    }
}

/// One line per stretch of `content` between line breaks.
fn push_lines(content: &[Inline], kind: LineKind, lines: &mut Vec<UserLine>) {
    for stretch in content.split(|inline| matches!(inline, Inline::HardBreak { .. })) {
        let content: Vec<Inline> = stretch.iter().map(without_origin).collect();
        let text = plain_text(&content);
        if !text.trim().is_empty() {
            lines.push(UserLine {
                kind,
                content,
                text: text.trim().to_owned(),
            });
        }
    }
}

fn without_origin(inline: &Inline) -> Inline {
    let mut inline = inline.clone();
    let marks = match &mut inline {
        Inline::Text { marks, .. }
        | Inline::HardBreak { marks }
        | Inline::Mention { marks, .. }
        | Inline::Citation { marks, .. } => marks,
    };
    marks.retain(|mark| !matches!(mark, Mark::Origin { .. }));
    inline
}

/// Text as a reader sees it: mentions as `@Name`, citations left out.
pub fn plain_text(content: &[Inline]) -> String {
    content
        .iter()
        .map(|inline| match inline {
            Inline::Text { text, .. } => text.clone(),
            Inline::Mention { attrs, .. } => format!("@{}", attrs.label),
            Inline::HardBreak { .. } => " ".to_owned(),
            Inline::Citation { .. } => String::new(),
        })
        .collect()
}

/// Finds which user line a copy is, each line claimable once.
pub struct LineMatcher {
    /// Each line's comparable text, and whether it is claimed.
    entries: Vec<(String, bool)>,
}

impl LineMatcher {
    /// A matcher over `lines`, none claimed yet.
    pub fn new(lines: &[UserLine]) -> Self {
        Self {
            entries: lines
                .iter()
                .map(|line| (comparable(&line.text), false))
                .collect(),
        }
    }

    /// Claims the unclaimed line `text` copies (exactly, ignoring case,
    /// spacing, list markers and Markdown emphasis), else the unclaimed line
    /// sharing at least [`FUZZY_MATCH`] of its words.
    pub fn claim(&mut self, text: &str) -> Option<usize> {
        let key = comparable(text);
        if key.is_empty() {
            return None;
        }
        let exact = self
            .entries
            .iter()
            .position(|(line, claimed)| !claimed && *line == key);
        let index = exact.or_else(|| self.closest(&key))?;
        if let Some(entry) = self.entries.get_mut(index) {
            entry.1 = true;
        }
        Some(index)
    }

    /// The lines nobody claimed, in order.
    pub fn unclaimed(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, (_, claimed))| !claimed)
            .map(|(index, _)| index)
            .collect()
    }

    /// Claims the unclaimed line `text` is exactly (ignoring case, spacing,
    /// list markers and emphasis), never a near copy: for AI text that
    /// happens to repeat a user line.
    pub fn claim_exact(&mut self, text: &str) -> Option<usize> {
        let key = comparable(text);
        let index = self
            .entries
            .iter()
            .position(|(line, claimed)| !claimed && !key.is_empty() && *line == key)?;
        if let Some(entry) = self.entries.get_mut(index) {
            entry.1 = true;
        }
        Some(index)
    }

    fn closest(&self, key: &str) -> Option<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, (_, claimed))| !claimed)
            .map(|(index, (line, _))| (index, word_overlap(key, line)))
            .filter(|(_, score)| *score >= FUZZY_MATCH)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }
}

/// Lower case, list markers and emphasis removed, spacing collapsed.
pub fn comparable(text: &str) -> String {
    let mut text = text.trim();
    for marker in [
        "- [ ] ", "- [x] ", "[ ] ", "[x] ", "[] ", "- ", "* ", "• ", "+ ",
    ] {
        if let Some(rest) = text.strip_prefix(marker) {
            text = rest.trim_start();
            break;
        }
    }
    text.replace(['*', '`'], "")
        .replace("~~", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Shared words over all words of either text (Jaccard), 0 to 1.
pub fn word_overlap(a: &str, b: &str) -> f64 {
    let words = |text: &str| -> std::collections::BTreeSet<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    let (a, b) = (words(a), words(b));
    let union = a.union(&b).count();
    if union == 0 {
        return 0.0;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "word counts of one line are far below 2^52"
    )]
    let score = a.intersection(&b).count() as f64 / union as f64;
    score
}

#[cfg(test)]
mod user_lines_tests;
