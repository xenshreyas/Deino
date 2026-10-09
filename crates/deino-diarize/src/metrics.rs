//! Diarization error rate (DER) for the benchmark harness.
//!
//! Ported from anarlog `crates/pyannote-local/src/metrics.rs` (MIT), which
//! follows pyannote.metrics: an optimal one-to-one speaker mapping
//! (Hungarian algorithm on overlap time) and a forgiveness collar around
//! reference boundaries (`collar` is the total width, half on each side).
//! Time is scored on a 10 ms grid; up to 64 speakers per side.

mod hungarian;

use std::collections::BTreeMap;
use std::time::Duration;

use deino_domain::Millis;

use crate::pipeline::SpeakerTurn;

/// Scoring resolution.
const BIN: u32 = 10;

/// One reference (ground-truth) turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceTurn {
    /// Start.
    pub start: Millis,
    /// End.
    pub end: Millis,
    /// Who spoke, as the reference names them.
    pub speaker: String,
}

/// The error breakdown, in seconds of reference speech.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DerReport {
    /// Reference speech scored.
    pub total: f64,
    /// Reference speech with no hypothesis speaker.
    pub missed: f64,
    /// Hypothesis speech beyond the reference.
    pub false_alarm: f64,
    /// Speech attributed to the wrong (mapped) speaker.
    pub confusion: f64,
    /// Speakers in the reference.
    pub reference_speakers: usize,
    /// Speakers in the hypothesis.
    pub hypothesis_speakers: usize,
}

impl DerReport {
    /// `(missed + false alarm + confusion) / total`, 0 for empty references.
    pub fn der(&self) -> f64 {
        if self.total <= 0.0 {
            return 0.0;
        }
        (self.missed + self.false_alarm + self.confusion) / self.total
    }
}

/// Speaker sets per 10 ms bin, as bitmasks.
struct Grid {
    reference: Vec<u64>,
    hypothesis: Vec<u64>,
    excluded: Vec<bool>,
}

/// Scores `hypothesis` against `reference` with a `collar`.
pub fn diarization_error_rate(
    reference: &[ReferenceTurn],
    hypothesis: &[SpeakerTurn],
    collar: Duration,
) -> DerReport {
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    for turn in reference {
        let next = names.len();
        names.entry(turn.speaker.as_str()).or_insert(next);
    }
    let hypothesis_speakers = hypothesis
        .iter()
        .map(|turn| turn.speaker + 1)
        .max()
        .unwrap_or(0);
    let grid = fill(reference, hypothesis, &names, collar);
    let overlap = overlap(&grid, names.len(), hypothesis_speakers);
    let mut mapped = vec![None; hypothesis_speakers];
    for (reference, hypothesis) in hungarian::max_weight_matching(&overlap) {
        if let Some(slot) = mapped.get_mut(hypothesis) {
            *slot = Some(reference);
        }
    }
    let mut report = score(&grid, &mapped);
    report.reference_speakers = names.len();
    report.hypothesis_speakers = hypothesis_speakers;
    report
}

fn bin_of(millis: Millis) -> usize {
    usize::try_from(millis.get().div_ceil(BIN)).unwrap_or(usize::MAX)
}

/// `millis` rounded to the nearest bin, saturating at 0.
fn round_bin(millis: i64) -> usize {
    usize::try_from((millis.max(0) + i64::from(BIN) / 2) / i64::from(BIN)).unwrap_or(0)
}

fn fill(
    reference: &[ReferenceTurn],
    hypothesis: &[SpeakerTurn],
    names: &BTreeMap<&str, usize>,
    collar: Duration,
) -> Grid {
    let end = reference
        .iter()
        .map(|t| t.end)
        .chain(hypothesis.iter().map(|t| t.end))
        .max();
    let bins = end.map_or(0, bin_of);
    let mut grid = Grid {
        reference: vec![0; bins],
        hypothesis: vec![0; bins],
        excluded: vec![false; bins],
    };
    let half = i64::try_from(collar.as_millis() / 2).unwrap_or(0);
    for turn in reference {
        let bit = 1_u64
            << names
                .get(turn.speaker.as_str())
                .copied()
                .unwrap_or(0)
                .min(63);
        mark(&mut grid.reference, span(turn.start, turn.end), bit);
        for boundary in [turn.start, turn.end] {
            let at = i64::from(boundary.get());
            let (from, to) = (round_bin(at - half), round_bin(at + half));
            grid.excluded
                .iter_mut()
                .take(to)
                .skip(from)
                .for_each(|slot| *slot = true);
        }
    }
    for turn in hypothesis {
        mark(
            &mut grid.hypothesis,
            span(turn.start, turn.end),
            1 << turn.speaker.min(63),
        );
    }
    grid
}

fn span(start: Millis, end: Millis) -> (usize, usize) {
    (
        round_bin(i64::from(start.get())),
        round_bin(i64::from(end.get())),
    )
}

fn mark(bins: &mut [u64], (from, to): (usize, usize), bit: u64) {
    bins.iter_mut()
        .take(to)
        .skip(from)
        .for_each(|slot| *slot |= bit);
}

/// Seconds each reference speaker overlaps each hypothesis speaker.
fn overlap(grid: &Grid, references: usize, hypotheses: usize) -> Vec<Vec<f64>> {
    let mut overlap = vec![vec![0.0; hypotheses]; references];
    let bins = grid
        .reference
        .iter()
        .zip(&grid.hypothesis)
        .zip(&grid.excluded);
    for ((reference, hypothesis), excluded) in bins {
        if *excluded {
            continue;
        }
        let rows = overlap
            .iter_mut()
            .enumerate()
            .filter(|(r, _)| reference & (1 << r) != 0);
        for (_, row) in rows {
            let cells = row
                .iter_mut()
                .enumerate()
                .filter(|(h, _)| hypothesis & (1 << h) != 0);
            cells.for_each(|(_, cell)| *cell += f64::from(BIN) / 1_000.0);
        }
    }
    overlap
}

/// Missed, false-alarm and confusion time under the speaker mapping.
fn score(grid: &Grid, mapped: &[Option<usize>]) -> DerReport {
    let step = f64::from(BIN) / 1_000.0;
    let mut report = DerReport::default();
    let bins = grid
        .reference
        .iter()
        .zip(&grid.hypothesis)
        .zip(&grid.excluded);
    for ((reference, hypothesis), excluded) in bins {
        if *excluded {
            continue;
        }
        let reference_size = f64::from(reference.count_ones());
        let active = mapped
            .iter()
            .enumerate()
            .filter(|(h, _)| hypothesis & (1 << h) != 0);
        let (hypothesis_size, correct) = active.fold((0.0, 0.0), |(size, correct), (_, map)| {
            let hit = map.is_some_and(|r| reference & (1 << r) != 0);
            (size + 1.0, correct + f64::from(u8::from(hit)))
        });
        report.total += reference_size * step;
        report.missed += (reference_size - hypothesis_size).max(0.0) * step;
        report.false_alarm += (hypothesis_size - reference_size).max(0.0) * step;
        report.confusion += (reference_size.min(hypothesis_size) - correct).max(0.0) * step;
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(start: u32, end: u32, speaker: &str) -> ReferenceTurn {
        ReferenceTurn {
            start: Millis(start),
            end: Millis(end),
            speaker: speaker.into(),
        }
    }

    fn segment(start: u32, end: u32, speaker: usize) -> SpeakerTurn {
        SpeakerTurn {
            start: Millis(start),
            end: Millis(end),
            speaker,
        }
    }

    #[test]
    fn a_perfect_hypothesis_scores_zero_under_any_labelling() {
        let reference = [turn(0, 5_000, "A"), turn(5_000, 10_000, "B")];
        let hypothesis = [segment(0, 5_000, 1), segment(5_000, 10_000, 0)];
        let report = diarization_error_rate(&reference, &hypothesis, Duration::ZERO);
        assert!(report.der().abs() < 1e-9, "{report:?}");
        assert!((report.total - 10.0).abs() < 1e-6);
    }

    #[test]
    fn missed_false_alarm_and_confusion_are_separated() {
        let reference = [turn(0, 4_000, "A"), turn(4_000, 8_000, "B")];
        let hypothesis = [
            segment(0, 2_000, 0),
            segment(2_000, 4_000, 1),
            segment(4_000, 8_000, 1),
            segment(8_000, 9_000, 1),
        ];
        let report = diarization_error_rate(&reference, &hypothesis, Duration::ZERO);
        assert!(report.missed.abs() < 1e-6);
        assert!((report.false_alarm - 1.0).abs() < 1e-6, "{report:?}");
        assert!((report.confusion - 2.0).abs() < 1e-6, "{report:?}");
        assert!((report.der() - 3.0 / 8.0).abs() < 1e-6);
    }

    #[test]
    fn the_collar_forgives_boundary_jitter() {
        let reference = [turn(0, 5_000, "A"), turn(5_000, 10_000, "B")];
        let hypothesis = [segment(0, 5_100, 0), segment(5_100, 10_000, 1)];
        assert!(diarization_error_rate(&reference, &hypothesis, Duration::ZERO).der() > 0.0);
        let lenient = diarization_error_rate(&reference, &hypothesis, Duration::from_millis(250));
        assert!(lenient.der().abs() < 1e-9, "{lenient:?}");
    }

    #[test]
    fn overlapping_reference_speech_counts_both_speakers() {
        let reference = [turn(0, 4_000, "A"), turn(2_000, 4_000, "B")];
        let report = diarization_error_rate(&reference, &[segment(0, 4_000, 0)], Duration::ZERO);
        assert!((report.total - 6.0).abs() < 1e-6);
        assert!((report.missed - 2.0).abs() < 1e-6, "{report:?}");
    }

    #[test]
    fn an_empty_hypothesis_is_all_missed() {
        let report = diarization_error_rate(&[turn(0, 3_000, "A")], &[], Duration::ZERO);
        assert!((report.der() - 1.0).abs() < 1e-9);
        assert_eq!(report.hypothesis_speakers, 0);
    }
}
