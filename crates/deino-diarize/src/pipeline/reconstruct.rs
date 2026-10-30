//! Stage 4: from labelled window activity to speaker turns.
//!
//! Every window votes its local speakers' activity onto the recording's
//! frame grid under their cluster labels; a speaker talks in a frame when at
//! least half the windows covering it say so. Runs of such frames become
//! turns, speakers without a turn are dropped, and the rest are numbered by
//! first appearance.
//!
//! With `exclusive` (Community-1's `exclusive_speaker_diarization`), each
//! frame has at most one speaker instead: when the windows covering it hear
//! about one or more people (their mean count of active local speakers,
//! rounded, is at least one), it goes to the speaker most windows hear.

use std::time::Duration;

use super::labels::Clusters;
use super::scan::{Scan, WindowGrid};
use super::{Diarization, DiarizedSpeaker, SpeakerTurn};
use crate::config::DiarizationConfig;
use crate::segmentation::{LOCAL_SPEAKERS, is_active};
use crate::units::{
    FRAME_SIZE, SAMPLE_RATE, WINDOW_SAMPLES, frame_start_sample, frames_for_samples,
    samples_to_millis,
};
use crate::vector::{Embedding, normalize};

/// A turn on the sample grid, before conversion to milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SampleTurn {
    start: usize,
    end: usize,
    speaker: usize,
}

/// Builds the result from the scan, the labels and the identities.
pub(super) fn reconstruct(
    scan: &Scan,
    grid: &WindowGrid,
    clusters: &Clusters,
    config: &DiarizationConfig,
) -> Diarization {
    let votes = Votes::cast(scan, grid, clusters);
    let exclusive = config.exclusive.then(|| votes.exclusive());
    let mut turns: Vec<SampleTurn> = (0..clusters.centroids.len())
        .flat_map(|speaker| {
            let active = match &exclusive {
                Some(owner) => owner.iter().map(|o| *o == Some(speaker)).collect(),
                None => votes.majority(speaker),
            };
            runs_to_turns(&active, speaker, config)
        })
        .map(|turn| SampleTurn {
            end: turn.end.min(grid.total),
            ..turn
        })
        .filter(|turn| turn.end > turn.start)
        .collect();
    turns.sort_by_key(|turn| (turn.start, turn.speaker));
    number_by_first_appearance(&turns, clusters)
}

/// Per-frame window counts: how many windows cover each frame, and how many
/// of them hear each speaker there.
struct Votes {
    coverage: Vec<u16>,
    /// Active local speakers per frame, summed over the covering windows.
    talkers: Vec<u16>,
    speakers: Vec<Vec<u16>>,
}

impl Votes {
    fn cast(scan: &Scan, grid: &WindowGrid, clusters: &Clusters) -> Self {
        let frames = frames_for_samples(grid.total.max(WINDOW_SAMPLES));
        let mut votes = Self {
            coverage: vec![0; frames],
            talkers: vec![0; frames],
            speakers: vec![vec![0; frames]; clusters.centroids.len()],
        };
        for (window, activity) in scan.activities.iter().enumerate() {
            let offset = grid.start(window) / FRAME_SIZE;
            increment(
                &mut votes.coverage,
                offset,
                activity.frames.iter().map(|_| true),
            );
            let talking = activity.frames.iter().map(|frame| {
                let count = (0..LOCAL_SPEAKERS).filter(|s| is_active(frame, *s)).count();
                u16::try_from(count).unwrap_or(u16::MAX)
            });
            add(&mut votes.talkers, offset, talking);
        }
        for (item, label) in scan.embeddings.iter().zip(&clusters.labels) {
            let (Some(activity), Some(track)) = (
                scan.activities.get(item.window),
                votes.speakers.get_mut(*label),
            ) else {
                continue;
            };
            let offset = grid.start(item.window) / FRAME_SIZE;
            let heard = activity
                .frames
                .iter()
                .map(|frame| is_active(frame, item.local_speaker));
            increment(track, offset, heard);
        }
        votes
    }

    /// Frames where at least half the covering windows hear `speaker`.
    fn majority(&self, speaker: usize) -> Vec<bool> {
        let empty = Vec::new();
        let track = self.speakers.get(speaker).unwrap_or(&empty);
        track
            .iter()
            .zip(&self.coverage)
            .map(|(votes, coverage)| *coverage > 0 && u32::from(*votes) * 2 >= u32::from(*coverage))
            .collect()
    }

    /// Each frame's one speaker, if anyone talks there: the most-heard
    /// speaker wherever the rounded mean count of talkers is at least one.
    fn exclusive(&self) -> Vec<Option<usize>> {
        (0..self.coverage.len())
            .map(|frame| {
                let coverage = f64::from(self.coverage.get(frame).copied().unwrap_or(0));
                let talkers = f64::from(self.talkers.get(frame).copied().unwrap_or(0));
                if coverage == 0.0 || (talkers / coverage).round_ties_even() < 1.0 {
                    return None;
                }
                let heard = |s: &Vec<u16>| s.get(frame).copied().unwrap_or(0);
                let (speaker, votes) = self
                    .speakers
                    .iter()
                    .enumerate()
                    .rev()
                    .max_by_key(|(_, track)| heard(track))?;
                (heard(votes) > 0).then_some(speaker)
            })
            .collect()
    }
}

/// Adds `values` to `counts` from `offset` on.
fn add(counts: &mut [u16], offset: usize, values: impl Iterator<Item = u16>) {
    for (slot, value) in counts.iter_mut().skip(offset).zip(values) {
        *slot = slot.saturating_add(value);
    }
}

/// Adds one to `counts[offset + i]` for every `true` in `hits`.
fn increment(counts: &mut [u16], offset: usize, hits: impl Iterator<Item = bool>) {
    for (slot, hit) in counts.iter_mut().skip(offset).zip(hits) {
        *slot += u16::from(hit);
    }
}

/// Runs of active frames as turns; gaps shorter than `min_duration_off`
/// are bridged first, then turns shorter than `min_duration_on` dropped.
fn runs_to_turns(active: &[bool], speaker: usize, config: &DiarizationConfig) -> Vec<SampleTurn> {
    let min_off = duration_samples(config.min_duration_off);
    let min_on = duration_samples(config.min_duration_on);
    let mut turns: Vec<SampleTurn> = Vec::new();
    let mut run_start: Option<usize> = None;
    for (frame, is_active) in active
        .iter()
        .copied()
        .chain(std::iter::once(false))
        .enumerate()
    {
        match (run_start, is_active) {
            (None, true) => run_start = Some(frame),
            (Some(start), false) => {
                let turn = SampleTurn {
                    start: frame_start_sample(start),
                    end: frame_start_sample(frame),
                    speaker,
                };
                match turns.last_mut() {
                    Some(previous) if turn.start - previous.end < min_off => {
                        previous.end = turn.end;
                    }
                    _ => turns.push(turn),
                }
                run_start = None;
            }
            _ => {}
        }
    }
    turns.retain(|turn| turn.end - turn.start >= min_on);
    turns
}

fn duration_samples(duration: Duration) -> usize {
    let samples = duration.as_millis() * u128::from(SAMPLE_RATE) / 1_000;
    usize::try_from(samples).unwrap_or(usize::MAX)
}

/// Renumbers speakers by their first turn and builds the result; speakers
/// without a turn are dropped.
fn number_by_first_appearance(turns: &[SampleTurn], clusters: &Clusters) -> Diarization {
    let (centroids, identities) = (&clusters.centroids, &clusters.identities);
    let mut order: Vec<usize> = Vec::new();
    for turn in turns {
        if !order.contains(&turn.speaker) {
            order.push(turn.speaker);
        }
    }
    let number = |speaker: usize| order.iter().position(|s| *s == speaker).unwrap_or(0);
    let speakers = order
        .iter()
        .enumerate()
        .filter_map(|(index, cluster)| {
            let centroid = Embedding::new(normalize(centroids.get(*cluster)?))?;
            let speech: usize = turns
                .iter()
                .filter(|turn| turn.speaker == *cluster)
                .map(|turn| turn.end - turn.start)
                .sum();
            let identity = identities.get(*cluster).copied().flatten();
            Some(DiarizedSpeaker {
                index,
                centroid,
                speech: samples_to_millis(speech),
                identity,
            })
        })
        .collect();
    let turns = turns
        .iter()
        .map(|turn| SpeakerTurn {
            start: samples_to_millis(turn.start),
            end: samples_to_millis(turn.end),
            speaker: number(turn.speaker),
        })
        .collect();
    Diarization { turns, speakers }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::scan::WindowEmbedding;
    use crate::segmentation::WindowActivity;

    fn grid(total: usize) -> WindowGrid {
        WindowGrid::new(total, &DiarizationConfig::default()).unwrap()
    }

    /// One window where local speaker 0 talks in frames `a` and speaker 1 in `b`.
    fn scan(a: std::ops::Range<usize>, b: std::ops::Range<usize>) -> Scan {
        let frames = (0..589)
            .map(|f| [a.contains(&f), b.contains(&f), false])
            .collect();
        let item = |local_speaker| WindowEmbedding {
            window: 0,
            local_speaker,
            embedding: vec![1.0, 0.0],
            anchor: true,
        };
        Scan {
            activities: vec![WindowActivity { frames }],
            embeddings: vec![item(0), item(1)],
        }
    }

    #[test]
    fn speakers_are_numbered_by_their_first_turn() {
        let clusters = Clusters {
            labels: vec![1, 0],
            centroids: vec![vec![0.0, 2.0], vec![3.0, 0.0]],
            identities: vec![None; 2],
        };
        let result = reconstruct(
            &scan(0..100, 200..300),
            &grid(160_000),
            &clusters,
            &DiarizationConfig::default(),
        );
        assert_eq!(result.turns.len(), 2);
        assert_eq!((result.turns[0].speaker, result.turns[1].speaker), (0, 1));
        assert_eq!(result.speakers[0].centroid.as_slice(), &[1.0, 0.0]);
        assert_eq!(result.speakers[1].centroid.as_slice(), &[0.0, 1.0]);
        assert_eq!(
            result.turns[0].start,
            samples_to_millis(frame_start_sample(0))
        );
        assert_eq!(
            result.speakers[0].speech,
            samples_to_millis(100 * FRAME_SIZE)
        );
    }

    #[test]
    fn short_gaps_are_bridged_and_short_turns_dropped() {
        let config = DiarizationConfig {
            min_duration_off: Duration::from_millis(500),
            min_duration_on: Duration::from_millis(500),
            ..DiarizationConfig::default()
        };
        let active: Vec<bool> = (0..200)
            .map(|f| (0..50).contains(&f) || (60..100).contains(&f) || (150..160).contains(&f))
            .collect();
        let turns = runs_to_turns(&active, 0, &config);
        assert_eq!(turns.len(), 1, "{turns:?}");
        assert_eq!(
            (turns[0].start, turns[0].end),
            (frame_start_sample(0), frame_start_sample(100))
        );
    }

    #[test]
    fn a_speaker_without_turns_is_dropped() {
        let clusters = Clusters {
            labels: vec![0, 0],
            centroids: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            identities: vec![None; 2],
        };
        let result = reconstruct(
            &scan(0..100, 0..0),
            &grid(160_000),
            &clusters,
            &DiarizationConfig::default(),
        );
        assert_eq!(result.speakers.len(), 1);
    }

    #[test]
    fn turns_end_at_the_end_of_the_audio() {
        let clusters = Clusters {
            labels: vec![0, 0],
            centroids: vec![vec![1.0, 0.0]],
            identities: vec![None; 2],
        };
        let result = reconstruct(
            &scan(0..589, 0..0),
            &grid(48_000),
            &clusters,
            &DiarizationConfig::default(),
        );
        assert_eq!(result.turns.last().unwrap().end, samples_to_millis(48_000));
    }

    #[test]
    fn exclusive_frames_go_to_the_most_heard_speaker_when_someone_talks() {
        // Four frames, two windows over each; speaker 1 is heard more in
        // frame 1, they tie in frame 2 (the lower index wins), and in
        // frame 3 one talker in two windows rounds to none (0.5 → 0).
        let votes = Votes {
            coverage: vec![2, 2, 2, 2],
            talkers: vec![2, 3, 2, 1],
            speakers: vec![vec![2, 1, 1, 1], vec![0, 2, 1, 0]],
        };

        assert_eq!(votes.exclusive(), vec![Some(0), Some(1), Some(0), None]);
    }
}
