//! Words and the longest common subsequence: what two texts (or two lists
//! of blocks) share, for the preview's word diff and for keeping the marks
//! of the words a rewrite kept.

use super::model::{DiffSegment, SegmentChange};

/// Above this many table cells the alignment keeps only the common prefix
/// and suffix (a rewrite of a whole long section).
const MAX_CELLS: usize = 1_000_000;

/// `text` as words (letters and digits), runs of whitespace, and single
/// other characters; joined, they give `text` back.
pub fn split_words(text: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut start = 0;
    let mut previous: Option<Class> = None;
    for (at, character) in text.char_indices() {
        let class = Class::of(character);
        let joins = previous == Some(class) && class != Class::Other;
        if !joins && at > start {
            words.extend(text.get(start..at));
            start = at;
        }
        previous = Some(class);
    }
    if start < text.len() {
        words.extend(text.get(start..));
    }
    words
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Word,
    Space,
    Other,
}

impl Class {
    fn of(character: char) -> Self {
        if character.is_alphanumeric() || character == '_' {
            Self::Word
        } else if character.is_whitespace() {
            Self::Space
        } else {
            Self::Other
        }
    }
}

/// The pairs `(i, j)` with `a[i] == b[j]` of a longest common subsequence,
/// in order.
pub fn common<T: PartialEq>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let rest_a = a.get(prefix..).unwrap_or_default();
    let rest_b = b.get(prefix..).unwrap_or_default();
    let suffix = rest_a
        .iter()
        .rev()
        .zip(rest_b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let middle_a = rest_a.get(..rest_a.len() - suffix).unwrap_or_default();
    let middle_b = rest_b.get(..rest_b.len() - suffix).unwrap_or_default();
    let mut pairs: Vec<(usize, usize)> = (0..prefix).map(|index| (index, index)).collect();
    if middle_a.len().saturating_mul(middle_b.len()) <= MAX_CELLS {
        let inner = table_pairs(middle_a, middle_b);
        pairs.extend(inner.into_iter().map(|(i, j)| (i + prefix, j + prefix)));
    }
    let tail_a = a.len() - suffix;
    let tail_b = b.len() - suffix;
    pairs.extend((0..suffix).map(|offset| (tail_a + offset, tail_b + offset)));
    pairs
}

/// The classic dynamic-programming LCS, walked back from the end.
fn table_pairs<T: PartialEq>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
    let width = b.len() + 1;
    let mut lengths = vec![0_u32; (a.len() + 1) * width];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            let value = if a.get(i) == b.get(j) {
                cell(&lengths, width, i + 1, j + 1) + 1
            } else {
                cell(&lengths, width, i + 1, j).max(cell(&lengths, width, i, j + 1))
            };
            if let Some(slot) = lengths.get_mut(i * width + j) {
                *slot = value;
            }
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut pairs = Vec::new();
    while i < a.len() && j < b.len() {
        if a.get(i) == b.get(j) {
            pairs.push((i, j));
            (i, j) = (i + 1, j + 1);
        } else if cell(&lengths, width, i + 1, j) >= cell(&lengths, width, i, j + 1) {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

fn cell(lengths: &[u32], width: usize, i: usize, j: usize) -> u32 {
    lengths.get(i * width + j).copied().unwrap_or(0)
}

/// `before` → `after` word by word: kept, removed and added runs, in order.
pub fn word_diff(before: &str, after: &str) -> Vec<DiffSegment> {
    let old = split_words(before);
    let new = split_words(after);
    let mut segments = Segments::default();
    let (mut i, mut j) = (0, 0);
    for (matched_i, matched_j) in common(&old, &new) {
        segments.extend(SegmentChange::Removed, old.get(i..matched_i));
        segments.extend(SegmentChange::Added, new.get(j..matched_j));
        segments.extend(SegmentChange::Same, old.get(matched_i..=matched_i));
        (i, j) = (matched_i + 1, matched_j + 1);
    }
    segments.extend(SegmentChange::Removed, old.get(i..));
    segments.extend(SegmentChange::Added, new.get(j..));
    segments.0
}

/// Segments, merging neighbours of the same change.
#[derive(Default)]
struct Segments(Vec<DiffSegment>);

impl Segments {
    fn extend(&mut self, change: SegmentChange, words: Option<&[&str]>) {
        let text: String = words.unwrap_or_default().concat();
        if text.is_empty() {
            return;
        }
        match self.0.last_mut() {
            Some(last) if last.change == change => last.text.push_str(&text),
            _ => self.0.push(DiffSegment { change, text }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_split_and_join_back() {
        let text = "Niall's plan: ship  v2.1!";
        let words = split_words(text);

        assert_eq!(
            words,
            [
                "Niall", "'", "s", " ", "plan", ":", " ", "ship", "  ", "v2", ".", "1", "!"
            ]
        );
        assert_eq!(words.concat(), text);
    }

    #[test]
    fn the_longest_common_subsequence_is_found() {
        let a = ["a", "b", "c", "d"];
        let b = ["b", "x", "d"];

        assert_eq!(common(&a, &b), [(1, 0), (3, 2)]);
    }

    #[test]
    fn a_word_diff_marks_what_changed() {
        let segments = word_diff("Niall owns pricing", "Nyal owns pricing now");
        let shown: Vec<(SegmentChange, &str)> = segments
            .iter()
            .map(|segment| (segment.change, segment.text.as_str()))
            .collect();

        assert_eq!(
            shown,
            [
                (SegmentChange::Removed, "Niall"),
                (SegmentChange::Added, "Nyal"),
                (SegmentChange::Same, " owns pricing"),
                (SegmentChange::Added, " now"),
            ]
        );
    }
}
