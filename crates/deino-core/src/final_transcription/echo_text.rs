//! Echo on the text side: Them's words that the microphone heard too.
//!
//! On laptop speakers the microphone track holds the other side of the
//! call. Echo cancellation removes most of it before the Me track is
//! transcribed, but not with every engine (the cloud hears the raw track)
//! and never all of it, so once both tracks are transcribed, Me lines that
//! repeat what Them said at the same moment are removed:
//!
//! 1. **Words** (when the engine timed them): the Me words are aligned with
//!    Them's words from the same moment (a longest common subsequence of
//!    normalised words, each pair within [`WORD_WINDOW`]), and each run of
//!    at least [`MIN_RUN`] matched words (allowing one misheard word
//!    between matches) is cut. The user's own words around it stay, and a
//!    reply that repeats a word or two of the question is never a run.
//! 2. **Whole segments**: what is left is dropped when it is Them's text
//!    from the same moment ([`overlap::overlaps`], similarity above 0.6,
//!    against up to [`MERGE`] adjacent Them segments merged, as the live
//!    holdback judges), Them was speaking around it, and most of its own
//!    words are in that text.
//!
//! A segment left without words is dropped. Only text decides: audio
//! correlation alone never removes anything (see `deino-recording`'s
//! `bleed`), because real double-talk correlates with the call's audio.

use deino_domain::{Channel, Millis};

use super::overlap;
use super::{CloudSpan, HeardSpan};
use crate::segments::WordTiming;

/// How far apart a Me word and the Them word it repeats may be.
pub const WORD_WINDOW: Millis = Millis(1_500);
/// How far apart a Me segment and a Them segment may start and be compared.
pub const SEGMENT_WINDOW: Millis = Millis(6_000);
/// Them text this close to a Me segment means Them was speaking around it.
pub const RISK_MARGIN: Millis = Millis(500);
/// Adjacent Them segments merged into one candidate text.
pub const MERGE: usize = 3;
/// The fewest matched words cut as echo.
pub const MIN_RUN: usize = 3;
/// Matched Me words at most this far apart (one word between) are one run.
const RUN_GAP: usize = 2;
/// The largest word alignment computed (Me words × Them words).
const MAX_CELLS: usize = 1_000_000;

/// A transcribed line the rule can read and rewrite.
pub trait Spoken {
    /// Its time and words.
    fn span(&self) -> &HeardSpan;
    /// Its time and words, to rewrite.
    fn span_mut(&mut self) -> &mut HeardSpan;
}

impl Spoken for HeardSpan {
    fn span(&self) -> &HeardSpan {
        self
    }

    fn span_mut(&mut self) -> &mut HeardSpan {
        self
    }
}

impl Spoken for CloudSpan {
    fn span(&self) -> &HeardSpan {
        &self.span
    }

    fn span_mut(&mut self) -> &mut HeardSpan {
        &mut self.span
    }
}

/// `heard` without the microphone's echo of the system track: Me lines
/// trimmed of Them's words, or dropped. Other channels are untouched.
pub fn without_echo<T: Spoken>(heard: Vec<(Channel, T)>) -> Vec<(Channel, T)> {
    let them = Them::of(&heard);
    if them.lines.is_empty() {
        return heard;
    }
    heard
        .into_iter()
        .filter_map(|(channel, mut line)| {
            if channel != Channel::Mic {
                return Some((channel, line));
            }
            *line.span_mut() = them.clean(line.span())?;
            Some((channel, line))
        })
        .collect()
}

/// One Them line, normalised.
struct ThemLine {
    start: Millis,
    end: Millis,
    tokens: Vec<String>,
    words: Vec<(Millis, String)>,
}

/// Everything Them said, in time order.
struct Them {
    lines: Vec<ThemLine>,
}

impl Them {
    fn of<T: Spoken>(heard: &[(Channel, T)]) -> Self {
        let mut lines: Vec<ThemLine> = heard
            .iter()
            .filter(|(channel, _)| *channel == Channel::System)
            .map(|(_, line)| ThemLine::of(line.span()))
            .collect();
        lines.sort_by_key(|line| line.start);
        Self { lines }
    }

    /// `me` without Them's words, or `None` when nothing of it is left.
    fn clean(&self, me: &HeardSpan) -> Option<HeardSpan> {
        let trimmed = self.trim_words(me)?;
        let tokens = overlap::tokens(&trimmed.text);
        if tokens.is_empty() || self.repeats(&trimmed, &tokens) {
            return None;
        }
        Some(trimmed)
    }

    /// `me` with each run of Them's words cut; `None` when no word is left.
    fn trim_words(&self, me: &HeardSpan) -> Option<HeardSpan> {
        if me.words.is_empty() {
            return Some(me.clone());
        }
        let from = me.start.0.saturating_sub(WORD_WINDOW.0);
        let to = me.end.0.saturating_add(WORD_WINDOW.0);
        let near: Vec<&(Millis, String)> = self
            .lines
            .iter()
            .flat_map(|line| &line.words)
            .filter(|(start, _)| (from..=to).contains(&start.0))
            .collect();
        let cut = echo_words(&me.words, &near);
        if cut.iter().all(|cut| !cut) {
            return Some(me.clone());
        }
        let kept: Vec<WordTiming> = me
            .words
            .iter()
            .zip(cut)
            .filter(|(_, cut)| !cut)
            .map(|(word, _)| word.clone())
            .collect();
        rebuilt(kept)
    }

    /// Whether `me` (with `tokens`) is Them's text from the same moment.
    fn repeats(&self, me: &HeardSpan, tokens: &[String]) -> bool {
        let nearby: Vec<&ThemLine> = self
            .lines
            .iter()
            .filter(|line| line.start.0.abs_diff(me.start.0) <= SEGMENT_WINDOW.0)
            .collect();
        let (from, to) = (
            me.start.0.saturating_sub(RISK_MARGIN.0),
            me.end.0.saturating_add(RISK_MARGIN.0),
        );
        let risky = nearby
            .iter()
            .any(|line| line.start.0 <= to && line.end.0 >= from);
        risky
            && candidates(&nearby).iter().any(|text| {
                overlap::overlaps(tokens, text)
                    && overlap::coverage(tokens, text) > overlap::SIMILARITY
            })
    }
}

impl ThemLine {
    fn of(span: &HeardSpan) -> Self {
        Self {
            start: span.start,
            end: span.end,
            tokens: overlap::tokens(&span.text),
            words: span
                .words
                .iter()
                .map(|word| (word.start, normalised(&word.text)))
                .filter(|(_, word)| !word.is_empty())
                .collect(),
        }
    }
}

/// Each run of up to [`MERGE`] consecutive Them lines, joined.
fn candidates(lines: &[&ThemLine]) -> Vec<Vec<String>> {
    let mut merged = Vec::new();
    for start in 0..lines.len() {
        let mut joined: Vec<String> = Vec::new();
        for line in lines.iter().skip(start).take(MERGE) {
            joined.extend(line.tokens.iter().cloned());
            merged.push(joined.clone());
        }
    }
    merged
}

/// A word as compared: lower case, letters and digits only.
fn normalised(word: &str) -> String {
    overlap::tokens(word).concat()
}

/// Which of `me`'s words to cut: those in runs of at least [`MIN_RUN`]
/// words matched with `them`, with the misheard words between them.
fn echo_words(me: &[WordTiming], them: &[&(Millis, String)]) -> Vec<bool> {
    let mut cut = vec![false; me.len()];
    if them.is_empty() || me.len().saturating_mul(them.len()) > MAX_CELLS {
        return cut;
    }
    let mine: Vec<(Millis, String)> = me
        .iter()
        .map(|word| (word.start, normalised(&word.text)))
        .collect();
    let matched = aligned(&mine, them);
    for run in runs(&matched) {
        if let (Some(first), Some(last)) = (run.first(), run.last()) {
            cut.iter_mut()
                .take(last + 1)
                .skip(*first)
                .for_each(|word| *word = true);
        }
    }
    cut
}

/// Whether Me word `a` and Them word `b` are the same word at the same time.
fn same(a: &(Millis, String), b: &(Millis, String)) -> bool {
    !a.1.is_empty() && a.1 == b.1 && a.0.0.abs_diff(b.0.0) <= WORD_WINDOW.0
}

/// The indices of `me` in a longest common subsequence with `them`.
fn aligned(me: &[(Millis, String)], them: &[&(Millis, String)]) -> Vec<usize> {
    let width = them.len() + 1;
    let mut table = vec![0_u32; (me.len() + 1) * width];
    let at = |i: usize, j: usize| i * width + j;
    for (i, a) in me.iter().enumerate().rev() {
        for (j, b) in them.iter().enumerate().rev() {
            let value = if same(a, b) {
                table.get(at(i + 1, j + 1)).map_or(0, |next| next + 1)
            } else {
                let down = table.get(at(i + 1, j)).copied().unwrap_or_default();
                let right = table.get(at(i, j + 1)).copied().unwrap_or_default();
                down.max(right)
            };
            if let Some(cell) = table.get_mut(at(i, j)) {
                *cell = value;
            }
        }
    }
    walk(me, them, &table, width)
}

/// Reads the matched pairs back out of the alignment table.
fn walk(
    me: &[(Millis, String)],
    them: &[&(Millis, String)],
    table: &[u32],
    width: usize,
) -> Vec<usize> {
    let cell = |i: usize, j: usize| table.get(i * width + j).copied().unwrap_or_default();
    let (mut i, mut j, mut matched) = (0, 0, Vec::new());
    while let (Some(a), Some(b)) = (me.get(i), them.get(j)) {
        if same(a, b) && cell(i, j) == cell(i + 1, j + 1) + 1 {
            matched.push(i);
            (i, j) = (i + 1, j + 1);
        } else if cell(i + 1, j) >= cell(i, j + 1) {
            i += 1;
        } else {
            j += 1;
        }
    }
    matched
}

/// `matched` (ascending) split into runs of at least [`MIN_RUN`] indices,
/// each at most [`RUN_GAP`] from the last.
fn runs(matched: &[usize]) -> Vec<Vec<usize>> {
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for &index in matched {
        match runs.last_mut() {
            Some(run) if run.last().is_some_and(|last| index - last <= RUN_GAP) => run.push(index),
            _ => runs.push(vec![index]),
        }
    }
    runs.retain(|run| run.len() >= MIN_RUN);
    runs
}

/// A line of the `kept` words, or `None` without any.
fn rebuilt(kept: Vec<WordTiming>) -> Option<HeardSpan> {
    let start = kept.first()?.start;
    let end = kept.last()?.end.max(start);
    let text = kept
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    Some(HeardSpan {
        start,
        end,
        text,
        words: kept,
    })
}

#[cfg(test)]
#[path = "echo_text_tests.rs"]
mod tests;
