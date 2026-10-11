//! Scores enhanced notes against the fixture they were written from.

use std::collections::BTreeSet;

use deino_doc::{Author, Block, Doc, Inline, Mark};
use deino_testkit::fixture::FixtureMeeting;

/// The share of AI lines that must cite the meeting.
const CITED_AI_LINES: f64 = 0.9;
/// The share of decisions the notes must cover.
const DECISIONS_COVERED: f64 = 0.75;
/// The share of facts the notes must cover.
const FACTS_COVERED: f64 = 0.5;
/// A statement is covered when the notes use this share of its keywords.
const KEYWORDS_USED: f64 = 0.5;
/// Words too common to show a statement is covered.
const STOP_WORDS: [&str; 12] = [
    "about", "after", "before", "their", "there", "these", "which", "would", "should", "could",
    "every", "other",
];

/// One line of the notes: who wrote it, its text, and whether it cites.
struct Line {
    author: Option<Author>,
    text: String,
    cites: bool,
}

/// The lines the user typed, as the engine reads them from their notes.
pub fn typed_lines(meeting: &FixtureMeeting) -> Vec<String> {
    let Some(markdown) = &meeting.user_notes else {
        return Vec::new();
    };
    deino_doc::from_markdown(markdown).map_or_else(
        |_| Vec::new(),
        |doc| {
            let lines = deino_core::enhance::user_lines(&doc);
            lines
                .into_iter()
                .map(|line| line.text)
                .filter(|text| !text.trim().is_empty())
                .collect()
        },
    )
}

/// Every check, reporting each that fails.
pub fn check(meeting: &FixtureMeeting, doc: &Doc) -> Result<(), String> {
    let segments: BTreeSet<String> = (0..meeting.segments.len())
        .map(|n| deino_testkit::fixture::fixture_uuid(&format!("segment/{}/{n}", meeting.id)))
        .collect();
    let lines = lines_of(doc, &segments);
    let failures: Vec<String> = [
        verbatim(meeting, &lines),
        cited(&lines),
        covered(
            "decisions",
            &meeting.ground_truth.decisions,
            &lines,
            DECISIONS_COVERED,
        ),
        covered("facts", &answers(meeting), &lines, FACTS_COVERED),
    ]
    .into_iter()
    .filter_map(Result::err)
    .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn answers(meeting: &FixtureMeeting) -> Vec<String> {
    meeting
        .ground_truth
        .facts
        .iter()
        .map(|fact| fact.answer.clone())
        .collect()
}

/// (a) Every typed line appears as the user's own text.
fn verbatim(meeting: &FixtureMeeting, lines: &[Line]) -> Result<(), String> {
    let theirs: Vec<String> = lines
        .iter()
        .filter(|line| line.author == Some(Author::User))
        .map(|line| squash(&line.text))
        .collect();
    let missing: Vec<String> = typed_lines(meeting)
        .into_iter()
        .filter(|typed| !theirs.iter().any(|text| text.contains(&squash(typed))))
        .collect();
    match missing.as_slice() {
        [] => Ok(()),
        _ => Err(format!(
            "{} typed lines are missing or reworded: {missing:?}",
            missing.len()
        )),
    }
}

/// (b) Enough AI lines cite a segment of the meeting.
fn cited(lines: &[Line]) -> Result<(), String> {
    let ai: Vec<&Line> = lines
        .iter()
        .filter(|line| line.author == Some(Author::Ai))
        .collect();
    let citing = ai.iter().filter(|line| line.cites).count();
    let share = ratio(citing, ai.len());
    (share >= CITED_AI_LINES)
        .then_some(())
        .ok_or(format!("{citing}/{} AI lines cite the meeting", ai.len()))
}

/// (c) Enough of `statements` are covered by the notes.
fn covered(what: &str, statements: &[String], lines: &[Line], bar: f64) -> Result<(), String> {
    let notes: BTreeSet<String> = lines.iter().flat_map(|line| keywords(&line.text)).collect();
    let uncovered: Vec<&String> = statements
        .iter()
        .filter(|statement| {
            let wanted = keywords(statement);
            ratio(wanted.intersection(&notes).count(), wanted.len()) < KEYWORDS_USED
        })
        .collect();
    let share = ratio(statements.len() - uncovered.len(), statements.len());
    (share >= bar)
        .then_some(())
        .ok_or(format!("{what} not covered: {uncovered:?}"))
}

#[allow(
    clippy::cast_precision_loss,
    reason = "counts of lines and statements are tiny"
)]
fn ratio(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        1.0
    } else {
        part as f64 / whole as f64
    }
}

fn keywords(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|word| word.chars().count() >= 5 && !STOP_WORDS.contains(&word.as_str()))
        .collect()
}

/// `text` with runs of whitespace as one space, trimmed.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every paragraph or heading with text, in reading order.
fn lines_of(doc: &Doc, segments: &BTreeSet<String>) -> Vec<Line> {
    fn walk(block: &Block, segments: &BTreeSet<String>, out: &mut Vec<Line>) {
        let inlines = block.inline_content();
        if inlines
            .iter()
            .any(|inline| matches!(inline, Inline::Text { .. }))
        {
            out.push(line_of(block, inlines, segments));
        }
        block
            .children()
            .into_iter()
            .for_each(|child| walk(child, segments, out));
    }
    let mut out = Vec::new();
    doc.content
        .iter()
        .for_each(|block| walk(block, segments, &mut out));
    out
}

/// A line's author (its first run's), text and citations; headings never cite.
fn line_of(block: &Block, inlines: &[Inline], segments: &BTreeSet<String>) -> Line {
    let author = inlines.iter().find_map(|inline| {
        inline.marks().iter().find_map(|mark| match mark {
            Mark::Origin { attrs } => Some(attrs.author),
            _ => None,
        })
    });
    let text = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let cites = inlines.iter().any(|inline| {
        matches!(inline, Inline::Citation { attrs, .. } if segments.contains(&attrs.segment_id.to_string()))
    });
    let is_heading = matches!(block, Block::Heading { .. });
    Line {
        author: if is_heading { None } else { author },
        text,
        cites,
    }
}
