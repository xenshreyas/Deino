//! CTC forced alignment of a line's letters to wav2vec2's frames (pure).
//!
//! A port of the former Python engine's `align_line`, `frames_by_word` and
//! `fill_gaps`, kept step for step so the two gave the same times:
//! uppercase letters and apostrophes become tokens, words are joined by the
//! `|` separator, a Viterbi pass over the log-probabilities emits each
//! token at one frame (blanks fill the rest), and a word spans its first to
//! its last letter's frame. Words with no letters (numbers, symbols) sit
//! between their neighbours.

/// wav2vec2-base-960h's vocabulary: `<pad>` (the CTC blank) is 0, `|`
/// (between words) is 4, then the letters by id.
const LETTERS: &str = "ETAONIHSRDLUMWCFGYPBVK'XJQZ";
/// The blank token.
pub const BLANK: usize = 0;
/// The word separator token.
const SEPARATOR: usize = 4;
/// The id of [`LETTERS`]' first letter.
const FIRST_LETTER: usize = 5;

/// A line's tokens, and the word each one belongs to (`None` for the
/// separators between words).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tokens {
    /// Vocabulary ids.
    pub ids: Vec<usize>,
    /// The word each id spells, by index into the line's words.
    pub owner: Vec<Option<usize>>,
}

/// The tokens spelling `words`, skipping words with no letters.
pub fn tokens(words: &[&str]) -> Tokens {
    let mut out = Tokens::default();
    for (index, word) in words.iter().enumerate() {
        let letters: Vec<usize> = word.to_uppercase().chars().filter_map(letter_id).collect();
        if letters.is_empty() {
            continue;
        }
        if !out.ids.is_empty() {
            out.ids.push(SEPARATOR);
            out.owner.push(None);
        }
        for id in letters {
            out.ids.push(id);
            out.owner.push(Some(index));
        }
    }
    out
}

fn letter_id(letter: char) -> Option<usize> {
    LETTERS.find(letter).map(|at| FIRST_LETTER + at)
}

/// The frame each token is emitted at: a Viterbi pass over `emission`
/// (log-probabilities, `frames × classes`, row-major) where a token is
/// emitted once and blanks fill every other frame. Needs at least as many
/// frames as tokens.
pub fn token_frames(emission: &[f32], classes: usize, ids: &[usize]) -> Vec<usize> {
    let rows: Vec<&[f32]> = emission.chunks_exact(classes.max(1)).collect();
    let table = trellis(&rows, ids);
    let mut at = vec![0; ids.len()];
    let mut j = ids.len();
    // Backwards from the last frame: frame `t` reads the table's row `t`
    // (the scores before it), as `speakers.py` does with `t - 1`.
    for (t, row) in rows.iter().enumerate().rev() {
        if j == 0 {
            break;
        }
        let before = table.get(t).map_or(&[][..], Vec::as_slice);
        let stay = cell(before, j) + prob(row, BLANK);
        let step = cell(before, j - 1) + prob(row, token(ids, j - 1));
        if step >= stay {
            if let Some(slot) = at.get_mut(j - 1) {
                *slot = t;
            }
            j -= 1;
        }
    }
    at
}

/// The best score of emitting the first `j` tokens in the first `t` frames:
/// `frames + 1` rows of `tokens + 1` scores.
fn trellis(rows: &[&[f32]], ids: &[usize]) -> Vec<Vec<f64>> {
    let mut origin = vec![f64::NEG_INFINITY; ids.len() + 1];
    if let Some(first) = origin.first_mut() {
        *first = 0.0;
    }
    let mut table = vec![origin];
    for row in rows {
        let next: Vec<f64> = {
            let before = table.last().map_or(&[][..], Vec::as_slice);
            let blank = prob(row, BLANK);
            (0..=ids.len())
                .map(|j| {
                    let stay = cell(before, j) + blank;
                    match j.checked_sub(1) {
                        Some(k) => stay.max(cell(before, k) + prob(row, token(ids, k))),
                        None => stay,
                    }
                })
                .collect()
        };
        table.push(next);
    }
    table
}

/// A table score, `-inf` outside it.
fn cell(row: &[f64], j: usize) -> f64 {
    row.get(j).copied().unwrap_or(f64::NEG_INFINITY)
}

/// A frame's log-probability of `class`, `-inf` outside it.
fn prob(row: &[f32], class: usize) -> f64 {
    row.get(class).map_or(f64::NEG_INFINITY, |&p| f64::from(p))
}

/// The `k`th token (the blank past the end, which never happens).
fn token(ids: &[usize], k: usize) -> usize {
    ids.get(k).copied().unwrap_or(BLANK)
}

/// Each word's first and last frame, by word index; words with no letters
/// get `None`.
pub fn word_frames(
    at: &[usize],
    owner: &[Option<usize>],
    words: usize,
) -> Vec<Option<(usize, usize)>> {
    let mut spans: Vec<Option<(usize, usize)>> = vec![None; words];
    for (&frame, word) in at.iter().zip(owner) {
        let Some(span) = word.and_then(|word| spans.get_mut(word)) else {
            continue;
        };
        *span = Some(span.map_or((frame, frame), |(first, last)| {
            (first.min(frame), last.max(frame))
        }));
    }
    spans
}

/// Fills the words the aligner could not place: each sits between its
/// placed neighbours, or the line's edges.
pub fn fill_gaps(times: &mut [Option<(f64, f64)>], line: (f64, f64)) {
    let known: Vec<(usize, (f64, f64))> = times
        .iter()
        .enumerate()
        .filter_map(|(index, time)| time.map(|time| (index, time)))
        .collect();
    for (index, time) in times.iter_mut().enumerate() {
        if time.is_some() {
            continue;
        }
        let before = known
            .iter()
            .rev()
            .find(|(at, _)| *at < index)
            .map(|(_, t)| t.1);
        let after = known.iter().find(|(at, _)| *at > index).map(|(_, t)| t.0);
        let start = before.unwrap_or(line.0);
        let end = after.unwrap_or(line.1);
        *time = Some((start, start.max(end)));
    }
}

#[cfg(test)]
#[path = "ctc_tests.rs"]
mod tests;
