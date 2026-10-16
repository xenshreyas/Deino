//! The word side of the echo rule: Me words aligned with Them's words from
//! the same moment, and the runs of matches that are cut.

use deino_domain::Millis;

use super::{MIN_RUN, WORD_WINDOW};
use crate::final_transcription::overlap;
use crate::segments::WordTiming;

/// Matched Me words at most this far apart (one word between) are one run.
const RUN_GAP: usize = 2;
/// The largest word alignment computed (Me words × Them words).
const MAX_CELLS: usize = 1_000_000;

/// A word as compared: lower case, letters and digits only.
pub(super) fn normalised(word: &str) -> String {
    overlap::tokens(word).concat()
}

/// Which of `me`'s words to cut: those in runs of at least [`MIN_RUN`]
/// words matched with `them`, with the misheard words between them.
pub(super) fn echo_words(me: &[WordTiming], them: &[&(Millis, String)]) -> Vec<bool> {
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
