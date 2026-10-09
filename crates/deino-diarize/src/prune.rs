//! Post-processing that removes speakers the evidence does not support.
//!
//! Ported from OpenWhispr `src/helpers/diarizationPolicy.js` (MIT), whose
//! thresholds come from field reports:
//!
//! - **Phantoms.** A cluster with under 1 s of speech is embedding noise.
//!   On longer recordings, backchannels and turn starts whose short
//!   embeddings land near no centroid form phantom speakers (a 20-minute
//!   two-person call produced four, holding 8 % of the speech): a phantom is
//!   both a sliver of the speech (under 10 %) and made of short turns (mean
//!   under 3 s). Merging two real people is the worse failure, so the two
//!   largest speakers are never phantoms. Known cost: a real third person
//!   with under 10 % of the speech in turns averaging under 3 s is dropped.
//! - **Collapse.** When one cluster holds over 93 % of the speech and every
//!   other one averages turns of 2.2 s or less, two voices were merged and
//!   the rest are phantoms: the labels are wrong either way, so the track
//!   becomes one speaker.

use std::time::Duration;

use deino_domain::Millis;

use crate::pipeline::{Diarization, SpeakerTurn};

/// Clusters with less speech than this are noise.
pub const MIN_SPEAKER_SPEECH: Duration = Duration::from_secs(1);
/// A phantom holds less than this share of the speech...
pub const PHANTOM_MAX_SHARE: f64 = 0.10;
/// ...in turns shorter than this on average.
pub const PHANTOM_MAX_MEAN_TURN: Duration = Duration::from_secs(3);
/// The largest speakers are never phantoms.
pub const PROTECTED_SPEAKERS: usize = 2;
/// A collapsed run's top speaker holds more than this share...
pub const COLLAPSED_TOP_SHARE: f64 = 0.93;
/// ...and every other speaker's turns average at most this.
pub const COLLAPSED_MAX_MEAN_TURN: Duration = Duration::from_millis(2_200);

/// Total speech and turn count of one speaker.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Talk {
    speech: f64,
    turns: usize,
}

impl Talk {
    fn mean_turn(self) -> f64 {
        self.speech / crate::units::as_f64(self.turns.max(1))
    }
}

/// Drops phantom speakers and their turns, then renumbers the rest by first
/// appearance. Leaves the result alone when every speaker, or none, would
/// stay.
pub fn prune_phantoms(diarization: Diarization) -> Diarization {
    let talk = talk_by_speaker(&diarization);
    let total: f64 = talk.iter().map(|t| t.speech).sum();
    let ranks = ranks(&talk);
    let keep: Vec<bool> = talk
        .iter()
        .zip(&ranks)
        .map(|(talk, rank)| is_supported(*talk, *rank, total))
        .collect();
    let kept = keep.iter().filter(|keep| **keep).count();
    if kept == 0 || kept == talk.len() {
        return diarization;
    }
    retain_speakers(diarization, &keep)
}

/// Whether one cluster swallowed the recording (see the module docs).
pub fn is_collapsed(diarization: &Diarization) -> bool {
    let min = MIN_SPEAKER_SPEECH.as_secs_f64();
    let mut heard: Vec<Talk> = talk_by_speaker(diarization)
        .into_iter()
        .filter(|talk| talk.speech >= min)
        .collect();
    if heard.len() < 2 {
        return false;
    }
    heard.sort_by(|a, b| b.speech.total_cmp(&a.speech));
    let total: f64 = heard.iter().map(|t| t.speech).sum();
    let Some((largest, rest)) = heard.split_first() else {
        return false;
    };
    largest.speech / total > COLLAPSED_TOP_SHARE
        && rest
            .iter()
            .all(|talk| talk.mean_turn() < COLLAPSED_MAX_MEAN_TURN.as_secs_f64())
}

/// Folds every speaker into the one who talks most: the answer for a
/// collapsed run. The kept centroid is the top speaker's.
pub fn collapse_to_one(diarization: Diarization) -> Diarization {
    let talk = talk_by_speaker(&diarization);
    let top = (0..talk.len())
        .max_by(|a, b| speech(&talk, *a).total_cmp(&speech(&talk, *b)))
        .unwrap_or(0);
    let keep: Vec<bool> = (0..talk.len()).map(|speaker| speaker == top).collect();
    let turns = diarization.turns.clone();
    let mut merged = retain_speakers(diarization, &keep);
    merged.turns = turns
        .iter()
        .map(|turn| SpeakerTurn {
            speaker: 0,
            ..*turn
        })
        .collect();
    merged.turns.sort_by_key(|turn| (turn.start, turn.end));
    if let Some(speaker) = merged.speakers.first_mut() {
        speaker.speech = Millis(
            merged
                .turns
                .iter()
                .map(|t| t.end.get() - t.start.get())
                .sum(),
        );
    }
    merged
}

fn speech(talk: &[Talk], speaker: usize) -> f64 {
    talk.get(speaker).map_or(0.0, |talk| talk.speech)
}

/// Whether a speaker of rank `rank` (0 = most speech) stays.
fn is_supported(talk: Talk, rank: usize, total: f64) -> bool {
    if talk.speech < MIN_SPEAKER_SPEECH.as_secs_f64() {
        return false;
    }
    rank < PROTECTED_SPEAKERS
        || talk.speech / total >= PHANTOM_MAX_SHARE
        || talk.mean_turn() >= PHANTOM_MAX_MEAN_TURN.as_secs_f64()
}

/// Each speaker's talk, indexed by speaker.
fn talk_by_speaker(diarization: &Diarization) -> Vec<Talk> {
    let mut talk = vec![Talk::default(); diarization.speakers.len()];
    for turn in &diarization.turns {
        if let Some(entry) = talk.get_mut(turn.speaker) {
            entry.speech += f64::from(turn.end.get() - turn.start.get()) / 1_000.0;
            entry.turns += 1;
        }
    }
    talk
}

/// Each speaker's rank by speech, most first (ties by index).
fn ranks(talk: &[Talk]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..talk.len()).collect();
    order.sort_by(|a, b| speech(talk, *b).total_cmp(&speech(talk, *a)));
    let mut ranks = vec![0; talk.len()];
    for (rank, speaker) in order.into_iter().enumerate() {
        if let Some(slot) = ranks.get_mut(speaker) {
            *slot = rank;
        }
    }
    ranks
}

/// Keeps the speakers marked in `keep` and their turns, renumbered by first
/// appearance.
fn retain_speakers(diarization: Diarization, keep: &[bool]) -> Diarization {
    let kept = |speaker: usize| keep.get(speaker).copied().unwrap_or(false);
    let mut order: Vec<usize> = Vec::new();
    for turn in diarization.turns.iter().filter(|turn| kept(turn.speaker)) {
        if !order.contains(&turn.speaker) {
            order.push(turn.speaker);
        }
    }
    let number = |speaker: usize| order.iter().position(|s| *s == speaker);
    let turns = diarization
        .turns
        .iter()
        .filter_map(|turn| {
            Some(SpeakerTurn {
                speaker: number(turn.speaker)?,
                ..*turn
            })
        })
        .collect();
    let mut speakers: Vec<_> = diarization
        .speakers
        .into_iter()
        .filter_map(|mut speaker| {
            speaker.index = number(speaker.index)?;
            Some(speaker)
        })
        .collect();
    speakers.sort_by_key(|speaker| speaker.index);
    Diarization { turns, speakers }
}

#[cfg(test)]
#[path = "prune_tests.rs"]
mod tests;
