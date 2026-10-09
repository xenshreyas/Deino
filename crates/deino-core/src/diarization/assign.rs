//! Attributing transcript segments to diarized voices (pure).
//!
//! Each word goes to the voice whose turns overlap it most, or, in a gap, to
//! the nearest turn (anarlog `pyannote-local::assign_words`, MIT). A segment
//! whose words change voice is split there. Flips shorter than
//! [`MIN_PIECE`] (a backchannel misheard at a turn boundary) join the
//! neighbouring piece instead of becoming a sliver of their own. Segments
//! without word timings, and segments the user edited (their words no
//! longer match the text), go whole to the voice that overlaps them most.

use deino_domain::Millis;

use super::VoiceTurn;
use crate::segments::{Segment, WordTiming};

/// The shortest stretch a segment is split into.
pub const MIN_PIECE: Millis = Millis(1_000);

/// One stretch of a segment and its voice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attributed {
    /// Index of the voice, `None` when the track has no turns at all.
    pub voice: Option<usize>,
    /// Start of the stretch.
    pub start: Millis,
    /// End of the stretch.
    pub end: Millis,
    /// Its text.
    pub text: String,
    /// Its words.
    pub words: Vec<WordTiming>,
}

/// The stretches of `segment` by voice, in time order (at least one).
pub fn attribute(segment: &Segment, turns: &[VoiceTurn]) -> Vec<Attributed> {
    if segment.words.is_empty() || segment.edited_at.is_some() {
        return vec![whole(segment, voice_at(segment.start, segment.end, turns))];
    }
    let labels: Vec<Option<usize>> = segment
        .words
        .iter()
        .map(|word| voice_at(word.start, word.end, turns))
        .collect();
    let runs = smooth(runs(&segment.words, &labels));
    if runs.len() <= 1 {
        let voice = runs.first().and_then(|run| run.voice);
        return vec![whole(segment, voice)];
    }
    pieces(segment, &runs)
}

/// The voice overlapping `[start, end)` most, else the nearest turn's.
pub fn voice_at(start: Millis, end: Millis, turns: &[VoiceTurn]) -> Option<usize> {
    let overlap = |turn: &VoiceTurn| {
        i64::from(turn.end.get().min(end.get())) - i64::from(turn.start.get().max(start.get()))
    };
    let distance = |turn: &VoiceTurn| {
        if turn.end <= start {
            start.get() - turn.end.get()
        } else {
            turn.start.get().saturating_sub(end.get())
        }
    };
    let best = turns.iter().filter(|turn| overlap(turn) > 0).fold(
        None,
        |best: Option<(i64, usize)>, turn| match best {
            Some((most, _)) if most >= overlap(turn) => best,
            _ => Some((overlap(turn), turn.voice)),
        },
    );
    best.map(|(_, voice)| voice).or_else(|| {
        turns
            .iter()
            .min_by_key(|turn| distance(turn))
            .map(|turn| turn.voice)
    })
}

/// Consecutive words of one voice: `words[first..=last]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run {
    voice: Option<usize>,
    first: usize,
    last: usize,
    length: u32,
}

fn runs(words: &[WordTiming], labels: &[Option<usize>]) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for (index, voice) in labels.iter().enumerate() {
        match runs.last_mut() {
            Some(run) if run.voice == *voice => run.last = index,
            _ => runs.push(Run {
                voice: *voice,
                first: index,
                last: index,
                length: 0,
            }),
        }
    }
    for run in &mut runs {
        let start = words.get(run.first).map_or(0, |word| word.start.get());
        let end = words.get(run.last).map_or(0, |word| word.end.get());
        run.length = end.saturating_sub(start);
    }
    runs
}

/// Folds runs shorter than [`MIN_PIECE`] into their previous neighbour
/// (the first into the next), then joins neighbours of the same voice.
fn smooth(runs: Vec<Run>) -> Vec<Run> {
    let mut kept: Vec<Run> = Vec::new();
    for run in runs {
        let only_one_kept = kept.len() == 1;
        match kept.last_mut() {
            Some(previous) if run.length < MIN_PIECE.get() || previous.voice == run.voice => {
                previous.last = run.last;
                previous.length += run.length;
            }
            Some(previous) if previous.length < MIN_PIECE.get() && only_one_kept => {
                *previous = Run {
                    first: previous.first,
                    length: previous.length + run.length,
                    ..run
                };
            }
            _ => kept.push(run),
        }
    }
    kept
}

/// One piece per run; the first starts and the last ends where the segment
/// does.
fn pieces(segment: &Segment, runs: &[Run]) -> Vec<Attributed> {
    let count = runs.len();
    runs.iter()
        .enumerate()
        .map(|(index, run)| {
            let words: Vec<WordTiming> = segment
                .words
                .get(run.first..=run.last)
                .unwrap_or_default()
                .to_vec();
            let start = if index == 0 {
                segment.start
            } else {
                words.first().map_or(segment.start, |w| w.start)
            };
            let end = if index + 1 == count {
                segment.end
            } else {
                words.last().map_or(segment.end, |w| w.end)
            };
            let text = words
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            Attributed {
                voice: run.voice,
                start,
                end,
                text,
                words,
            }
        })
        .collect()
}

fn whole(segment: &Segment, voice: Option<usize>) -> Attributed {
    Attributed {
        voice,
        start: segment.start,
        end: segment.end,
        text: segment.text.clone(),
        words: segment.words.clone(),
    }
}

#[cfg(test)]
#[path = "assign_tests.rs"]
mod tests;
