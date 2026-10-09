use deino_domain::Millis;

use super::*;
use crate::pipeline::DiarizedSpeaker;
use crate::vector::Embedding;

/// A diarization from `(speaker, start_ms, end_ms)` turns; speaker `n`'s
/// centroid is `[n, 1]`.
fn diarization(turns: &[(usize, u32, u32)]) -> Diarization {
    let count = turns.iter().map(|t| t.0 + 1).max().unwrap_or(0);
    let centroid = |index: usize| {
        let x = (0..index).fold(0.0_f32, |x, _| x + 1.0);
        Embedding::new(vec![x, 1.0]).unwrap()
    };
    Diarization {
        turns: turns
            .iter()
            .map(|&(speaker, start, end)| SpeakerTurn {
                start: Millis(start),
                end: Millis(end),
                speaker,
            })
            .collect(),
        speakers: (0..count)
            .map(|index| DiarizedSpeaker {
                index,
                centroid: centroid(index),
                speech: Millis(0),
                identity: None,
            })
            .collect(),
    }
}

/// `count` turns of `speaker`, `length` ms each, every `every` ms from `from`.
fn bursts(
    speaker: usize,
    from: u32,
    every: u32,
    (count, length): (usize, u32),
) -> Vec<(usize, u32, u32)> {
    (from..)
        .step_by(usize::try_from(every).unwrap())
        .take(count)
        .map(|start| (speaker, start, start + length))
        .collect()
}

fn speakers_of(result: &Diarization) -> Vec<usize> {
    result.turns.iter().map(|turn| turn.speaker).collect()
}

#[test]
fn a_speaker_with_under_a_second_of_speech_is_noise() {
    let result = prune_phantoms(diarization(&[
        (0, 0, 30_000),
        (1, 30_000, 60_000),
        (2, 61_000, 61_500),
    ]));
    assert_eq!(speakers_of(&result), vec![0, 1]);
    assert_eq!(result.speakers.len(), 2);
}

#[test]
fn a_sliver_of_short_turns_beyond_the_top_two_is_a_phantom() {
    let mut turns = vec![(0, 0, 100_000), (1, 100_000, 200_000)];
    turns.extend(bursts(2, 200_000, 3_000, (5, 1_500)));
    let result = prune_phantoms(diarization(&turns));
    assert_eq!(result.speakers.len(), 2);
    assert!(speakers_of(&result).iter().all(|speaker| *speaker < 2));
}

#[test]
fn a_minor_speaker_with_long_turns_stays() {
    let turns = [
        (0, 0, 100_000),
        (1, 100_000, 200_000),
        (2, 200_000, 210_000),
    ];
    assert_eq!(prune_phantoms(diarization(&turns)).speakers.len(), 3);
}

#[test]
fn the_two_largest_speakers_are_never_pruned() {
    let mut turns = vec![(0, 0, 300_000)];
    turns.extend(bursts(1, 300_000, 2_000, (10, 1_500)));
    assert_eq!(prune_phantoms(diarization(&turns)).speakers.len(), 2);
}

#[test]
fn pruned_speakers_are_renumbered_by_first_appearance() {
    let turns = [(0, 500, 600), (1, 1_000, 50_000), (2, 50_000, 100_000)];
    let result = prune_phantoms(diarization(&turns));
    assert_eq!(speakers_of(&result), vec![0, 1]);
    assert_eq!(
        result.speakers.iter().map(|s| s.index).collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        result.speakers[0].centroid,
        diarization(&turns).speakers[1].centroid,
        "speaker 1 became 0"
    );
}

#[test]
fn a_dominant_speaker_beside_backchannels_is_a_collapse() {
    let mut turns = vec![(0, 0, 300_000)];
    turns.extend(bursts(1, 300_000, 3_000, (5, 1_500)));
    assert!(is_collapsed(&diarization(&turns)));
    let collapsed = collapse_to_one(diarization(&turns));
    assert_eq!(collapsed.speakers.len(), 1);
    assert!(speakers_of(&collapsed).iter().all(|speaker| *speaker == 0));
    assert_eq!(collapsed.turns.len(), 6);
}

#[test]
fn an_interview_with_real_questions_is_not_a_collapse() {
    let mut turns = vec![(0, 0, 300_000)];
    turns.extend(bursts(1, 300_000, 5_000, (4, 4_000)));
    assert!(!is_collapsed(&diarization(&turns)));
    assert!(!is_collapsed(&diarization(&[(0, 0, 10_000)])));
}
