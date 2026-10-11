//! [`Corrector`]: applies a [`Lexicon`] to transcript text and words.
//!
//! A segment's text and its timed words are corrected separately with the
//! same rules, so both read the same. A correction replaces whole word cores
//! and keeps the punctuation around them; replaced words keep the time they
//! covered, shared evenly among the words that replace them. Words that do
//! not split into one token each are left as they are.

use deino_domain::Millis;

use super::Lexicon;
use super::matcher::{Fix, find};
use super::tokens::{Token, tokenize};
use crate::segments::WordTiming;

/// Corrects transcript text with the vocabulary. Cheap enough to run on
/// every live segment: matching is a hash lookup per word plus a fuzzy pass
/// over the names sharing the word's first letter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Corrector {
    lexicon: Lexicon,
}

/// A segment's corrected text and words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Corrected {
    /// The corrected text.
    pub text: String,
    /// The corrected words.
    pub words: Vec<WordTiming>,
}

impl Corrector {
    /// A corrector over `lexicon`.
    pub fn new(lexicon: Lexicon) -> Self {
        Self { lexicon }
    }

    /// True when it never corrects anything.
    pub fn is_empty(&self) -> bool {
        self.lexicon.is_empty()
    }

    /// `text` corrected, or `None` when nothing changes.
    pub fn text(&self, text: &str) -> Option<String> {
        let tokens = tokenize(text);
        let fixes = find(text, &tokens, &self.lexicon);
        (!fixes.is_empty()).then(|| apply_text(text, &tokens, &fixes))
    }

    /// A segment's `text` and `words` corrected, or `None` when the text
    /// does not change.
    pub fn segment(&self, text: &str, words: &[WordTiming]) -> Option<Corrected> {
        let text = self.text(text)?;
        Some(Corrected {
            text,
            words: self.words(words).unwrap_or_else(|| words.to_vec()),
        })
    }

    /// `words` corrected, or `None` when nothing changes or a word is not
    /// exactly one token.
    fn words(&self, words: &[WordTiming]) -> Option<Vec<WordTiming>> {
        let line = words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let tokens = tokenize(&line);
        if tokens.len() != words.len() {
            return None;
        }
        let fixes = find(&line, &tokens, &self.lexicon);
        (!fixes.is_empty()).then(|| {
            apply_words(
                &Line {
                    text: &line,
                    tokens: &tokens,
                },
                words,
                &fixes,
            )
        })
    }
}

/// `text` with `fixes` (in order, never overlapping) applied.
fn apply_text(text: &str, tokens: &[Token], fixes: &[Fix]) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut cursor = 0;
    for fix in fixes {
        let (Some(first), Some(last)) = (tokens.get(fix.tokens.start), last_of(tokens, fix)) else {
            continue;
        };
        out.push_str(text.get(cursor..first.core.start).unwrap_or_default());
        out.push_str(&fix.text);
        cursor = last.core.end;
    }
    out.push_str(text.get(cursor..).unwrap_or_default());
    out
}

/// The words joined by spaces, and its tokens (one per word).
struct Line<'a> {
    text: &'a str,
    tokens: &'a [Token],
}

/// `words` with `fixes` applied; fix ranges index words, one token each.
fn apply_words(line: &Line<'_>, words: &[WordTiming], fixes: &[Fix]) -> Vec<WordTiming> {
    let mut out = Vec::with_capacity(words.len());
    let mut cursor = 0;
    for fix in fixes {
        out.extend(
            words
                .get(cursor..fix.tokens.start)
                .unwrap_or_default()
                .iter()
                .cloned(),
        );
        out.extend(replaced(line, words, fix));
        cursor = fix.tokens.end;
    }
    out.extend(words.get(cursor..).unwrap_or_default().iter().cloned());
    out
}

/// The words that replace `fix`'s words: its text's words, keeping the
/// punctuation before the first and after the last, sharing their time.
fn replaced(line: &Line<'_>, words: &[WordTiming], fix: &Fix) -> Vec<WordTiming> {
    let (Some(first), Some(last)) = (line.tokens.get(fix.tokens.start), last_of(line.tokens, fix))
    else {
        return Vec::new();
    };
    let before = line
        .text
        .get(first.span.start..first.core.start)
        .unwrap_or_default();
    let after = line
        .text
        .get(last.core.end..last.span.end)
        .unwrap_or_default();
    let start = words.get(fix.tokens.start).map_or(Millis(0), |w| w.start);
    let end = fix
        .tokens
        .end
        .checked_sub(1)
        .and_then(|i| words.get(i))
        .map_or(start, |w| w.end);
    let parts: Vec<&str> = fix.text.split_whitespace().collect();
    let count = parts.len();
    parts
        .iter()
        .enumerate()
        .map(|(index, part)| WordTiming {
            text: decorated(
                part,
                (index == 0).then_some(before),
                (index + 1 == count).then_some(after),
            ),
            start: share(start, end, index, count),
            end: share(start, end, index + 1, count),
        })
        .collect()
}

fn decorated(part: &str, before: Option<&str>, after: Option<&str>) -> String {
    format!(
        "{}{part}{}",
        before.unwrap_or_default(),
        after.unwrap_or_default()
    )
}

/// The `index`-th of `count` even cuts of `start..end`.
fn share(start: Millis, end: Millis, index: usize, count: usize) -> Millis {
    let span = u64::from(end.0.saturating_sub(start.0));
    let index = u64::try_from(index).unwrap_or(u64::MAX);
    let count = u64::try_from(count.max(1)).unwrap_or(1);
    let offset = u32::try_from(span.saturating_mul(index) / count).unwrap_or(u32::MAX);
    Millis(start.0.saturating_add(offset).min(end.0))
}

fn last_of<'a>(tokens: &'a [Token], fix: &Fix) -> Option<&'a Token> {
    fix.tokens.end.checked_sub(1).and_then(|i| tokens.get(i))
}

#[cfg(test)]
#[path = "corrector_tests.rs"]
mod tests;
