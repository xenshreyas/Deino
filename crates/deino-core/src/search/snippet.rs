//! Snippets: text with the ranges that matched, for the UI to highlight.
//!
//! Keyword hits come back from FTS5's `highlight()` and `snippet()` with the
//! matches wrapped in two control characters ([`OPEN`], [`CLOSE`]), which
//! [`from_marked`] strips into [`Highlight`] ranges. Semantic hits have no
//! FTS5 marks, so [`excerpt`] finds the query's words itself. Ranges are in
//! UTF-16 code units, the unit of JavaScript string indices.

use super::{Highlight, Snippet};

/// Opens a match in FTS5 output (STX, never typed by a person).
pub const OPEN: char = '\u{2}';
/// Closes a match in FTS5 output (ETX).
pub const CLOSE: char = '\u{3}';
/// The longest excerpt [`excerpt`] makes, in characters.
pub const MAX_EXCERPT_CHARS: usize = 220;
/// Characters kept before the first match in an [`excerpt`].
const LEAD_CHARS: usize = 60;
/// What marks text cut from an excerpt (FTS5 `snippet()` uses the same).
pub const ELLIPSIS: &str = "…";

/// Strips FTS5's match marks from `marked` into highlight ranges. A mark
/// left open at the end closes there; stray closing marks are dropped.
pub fn from_marked(marked: &str) -> Snippet {
    let mut text = String::with_capacity(marked.len());
    let mut highlights = Vec::new();
    let mut position = 0_u32;
    let mut open: Option<u32> = None;
    for character in marked.chars() {
        match character {
            OPEN => open = open.or(Some(position)),
            CLOSE => close(&mut highlights, open.take(), position),
            _ => {
                text.push(character);
                position += utf16_len(character);
            }
        }
    }
    close(&mut highlights, open, position);
    Snippet { text, highlights }
}

fn close(highlights: &mut Vec<Highlight>, start: Option<u32>, end: u32) {
    if let Some(start) = start.filter(|start| *start < end) {
        highlights.push(Highlight { start, end });
    }
}

fn utf16_len(character: char) -> u32 {
    // A char is one or two UTF-16 code units.
    u32::try_from(character.len_utf16()).unwrap_or(2)
}

/// At most [`MAX_EXCERPT_CHARS`] of `text` starting a little before the first
/// word that matches one of `words` (lower case), with every matching word
/// highlighted. A word matches when it starts with a query word, or is a
/// prefix of one at least four letters long (a rough stand-in for the
/// stemming FTS5 does).
pub fn excerpt(text: &str, words: &[String]) -> Snippet {
    let spans = word_spans(text);
    let first = spans
        .iter()
        .find(|span| matches_any(&text[span.0..span.1], words));
    let start = first.map_or(0, |span| back_to_word_start(text, span.0));
    let mut window: String = text[start..].chars().take(MAX_EXCERPT_CHARS).collect();
    let cut_end = window.len() < text.len() - start;
    let prefix = if start > 0 { ELLIPSIS } else { "" };
    if cut_end {
        window.push_str(ELLIPSIS);
    }
    let marked = mark(&format!("{prefix}{window}"), words);
    from_marked(&marked)
}

/// The line of `text` with the most words matching `words` (the first line
/// when none match), for showing the relevant turn of a transcript window.
pub fn best_line<'a>(text: &'a str, words: &[String]) -> &'a str {
    let matches = |line: &str| {
        word_spans(line)
            .into_iter()
            .filter(|(start, end)| matches_any(&line[*start..*end], words))
            .count()
    };
    let mut best = text.lines().next().unwrap_or_default();
    let mut most = 0;
    for line in text.lines() {
        let count = matches(line);
        if count > most {
            (best, most) = (line, count);
        }
    }
    best
}

/// `text` with each word that matches one of `words` wrapped in marks.
fn mark(text: &str, words: &[String]) -> String {
    let mut marked = String::with_capacity(text.len() + 8);
    let mut copied = 0;
    for (start, end) in word_spans(text) {
        if matches_any(&text[start..end], words) {
            marked.push_str(&text[copied..start]);
            marked.push(OPEN);
            marked.push_str(&text[start..end]);
            marked.push(CLOSE);
            copied = end;
        }
    }
    marked.push_str(&text[copied..]);
    marked
}

fn matches_any(word: &str, words: &[String]) -> bool {
    let word = word.to_lowercase();
    words.iter().any(|query| {
        word.starts_with(query.as_str())
            || (word.chars().count() >= 4 && query.starts_with(word.as_str()))
    })
}

/// The byte ranges of the alphanumeric runs in `text`.
fn word_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start = None;
    for (index, character) in text.char_indices() {
        match (character.is_alphanumeric(), start) {
            (true, None) => start = Some(index),
            (false, Some(begin)) => {
                spans.push((begin, index));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(begin) = start {
        spans.push((begin, text.len()));
    }
    spans
}

/// The start of the word at most [`LEAD_CHARS`] characters before `index`.
fn back_to_word_start(text: &str, index: usize) -> usize {
    let lead: Vec<(usize, char)> = text[..index].char_indices().collect();
    let Some(from) = lead.len().checked_sub(LEAD_CHARS) else {
        return 0;
    };
    lead.iter()
        .skip(from)
        .find(|(_, character)| character.is_whitespace())
        .map_or(index, |(position, character)| {
            position + character.len_utf8()
        })
}

#[cfg(test)]
#[path = "snippet_tests.rs"]
mod tests;
