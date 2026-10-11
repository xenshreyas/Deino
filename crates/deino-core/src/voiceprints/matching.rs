//! Voiceprint matching primitives: who is this voice, conservatively.
//!
//! Adapted from anarlog `crates/voiceprint/src/matching.rs` (MIT), made
//! generic over the speaker and person keys so CAP-04 can pair Deino's
//! `SpeakerId`s with `PersonId`s. A pairing needs **both** a score above the
//! floor and a margin over the runner-up, **in both directions** (mutual
//! best): end to end in the spike, a phantom cluster contaminated with
//! another voice matched the wrong person at 0.81, and only the two
//! safeguards together rejected it (`docs/references/spike-diarization.md`).
//!
//! CAP-03 shipped these in `deino-diarize`; CAP-04 moved them here, beside
//! the `identify_speakers` rules that use them (ADR 0028). The engine's
//! known-speaker merge imports them from here.

use std::collections::BTreeMap;

/// anarlog's floor for a unique match with WeSpeaker ResNet34.
pub const MIN_UNIQUE_SCORE: f32 = 0.62;

/// anarlog's margin over the runner-up for a unique match.
pub const MIN_UNIQUE_MARGIN: f32 = 0.08;

/// What a pairing must clear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MatchRule {
    /// The best score must reach this.
    pub min_score: f32,
    /// ...and beat the runner-up by this much.
    pub min_margin: f32,
}

impl Default for MatchRule {
    fn default() -> Self {
        Self {
            min_score: MIN_UNIQUE_SCORE,
            min_margin: MIN_UNIQUE_MARGIN,
        }
    }
}

/// A speaker paired with a person, and the score that decided it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Assignment<S, P> {
    /// The voice.
    pub speaker: S,
    /// Who it is.
    pub person: P,
    /// Their similarity.
    pub score: f32,
}

/// A person's score for a voice: the best cosine similarity over their
/// exemplars (anarlog keeps up to five; the max, not the mean, so one good
/// exemplar is enough). `None` when no exemplar is comparable.
pub fn best_score<'a>(
    voice: &[f32],
    exemplars: impl IntoIterator<Item = &'a [f32]>,
) -> Option<f32> {
    exemplars
        .into_iter()
        .filter_map(|exemplar| cosine(voice, exemplar))
        .max_by(f32::total_cmp)
}

/// Cosine similarity in `[-1, 1]`; `None` for mismatched lengths, empty or
/// zero vectors.
pub fn cosine(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }
    let dot: f32 = left.iter().zip(right).map(|(l, r)| l * r).sum();
    let norm = |vector: &[f32]| vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    let denominator = norm(left) * norm(right);
    (denominator > 0.0).then(|| (dot / denominator).clamp(-1.0, 1.0))
}

/// The pairings that are each side's unique best under `rule`, sorted by
/// speaker then person. `scores` holds one `(speaker, person, score)` per
/// comparable pair.
pub fn pick_unique<S: Copy + Ord, P: Copy + Ord>(
    scores: &[(S, P, f32)],
    rule: MatchRule,
) -> Vec<Assignment<S, P>> {
    let mut by_speaker: BTreeMap<S, Ranking<P>> = BTreeMap::new();
    let mut by_person: BTreeMap<P, Ranking<S>> = BTreeMap::new();
    for &(speaker, person, score) in scores {
        by_speaker.entry(speaker).or_default().offer(person, score);
        by_person.entry(person).or_default().offer(speaker, score);
    }
    by_speaker
        .into_iter()
        .filter_map(|(speaker, ranking)| {
            let (person, score) = ranking.unique(rule)?;
            let (back, _) = by_person.get(&person)?.unique(rule)?;
            (back == speaker).then_some(Assignment {
                speaker,
                person,
                score,
            })
        })
        .collect()
}

/// When a channel holds exactly as many speakers as people who could be on
/// it, and all but one of each are paired, the last speaker is the last
/// person. A no-show or an uninvited guest breaks the count, so nothing is
/// guessed then.
pub fn complete_by_elimination<S: Copy + PartialEq, P: Copy + PartialEq>(
    speakers: &[S],
    people: &[P],
    assigned: &[(S, P)],
) -> Option<(S, P)> {
    if speakers.is_empty() || speakers.len() != people.len() {
        return None;
    }
    let mut open_speakers = speakers
        .iter()
        .filter(|speaker| !assigned.iter().any(|(s, _)| s == *speaker));
    let mut open_people = people
        .iter()
        .filter(|person| !assigned.iter().any(|(_, p)| p == *person));
    let (speaker, person) = (open_speakers.next()?, open_people.next()?);
    if open_speakers.next().is_some() || open_people.next().is_some() {
        return None;
    }
    Some((*speaker, *person))
}

/// The best and runner-up scores one key has seen.
struct Ranking<K> {
    best: Option<(K, f32)>,
    second: f32,
}

impl<K> Default for Ranking<K> {
    fn default() -> Self {
        Self {
            best: None,
            second: f32::NEG_INFINITY,
        }
    }
}

impl<K: Copy + PartialEq> Ranking<K> {
    fn offer(&mut self, key: K, score: f32) {
        match self.best {
            Some((best, best_score)) if score > best_score => {
                if best != key {
                    self.second = best_score;
                }
                self.best = Some((key, score));
            }
            Some((best, _)) if best != key && score > self.second => self.second = score,
            Some(_) => {}
            None => self.best = Some((key, score)),
        }
    }

    /// The best key when it clears the floor and the margin.
    fn unique(&self, rule: MatchRule) -> Option<(K, f32)> {
        let (key, best) = self.best?;
        let clear = self.second.is_infinite() || best - self.second >= rule.min_margin;
        (best >= rule.min_score && clear).then_some((key, best))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULE: MatchRule = MatchRule {
        min_score: 0.62,
        min_margin: 0.08,
    };

    #[test]
    fn a_clear_mutual_best_is_assigned() {
        let scores = [(0, 'a', 0.9), (0, 'b', 0.3), (1, 'a', 0.2), (1, 'b', 0.8)];
        let assigned = pick_unique(&scores, RULE);
        assert_eq!(
            assigned,
            vec![
                Assignment {
                    speaker: 0,
                    person: 'a',
                    score: 0.9
                },
                Assignment {
                    speaker: 1,
                    person: 'b',
                    score: 0.8
                },
            ]
        );
    }

    #[test]
    fn a_narrow_margin_or_low_score_is_not_assigned() {
        assert!(pick_unique(&[(0, 'a', 0.80), (0, 'b', 0.75)], RULE).is_empty());
        assert!(pick_unique(&[(0, 'a', 0.5)], RULE).is_empty());
        assert_eq!(pick_unique(&[(0, 'a', 0.7)], RULE).len(), 1);
    }

    #[test]
    fn a_person_claimed_by_two_voices_goes_to_neither_unless_one_is_clear() {
        // Both voices prefer `a`; `a` cannot tell them apart.
        assert!(pick_unique(&[(0, 'a', 0.81), (1, 'a', 0.80)], RULE).is_empty());
        // `a` clearly prefers voice 1; voice 0 is left unassigned.
        let assigned = pick_unique(&[(0, 'a', 0.70), (1, 'a', 0.90)], RULE);
        assert_eq!(
            assigned.iter().map(|a| a.speaker).collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn best_score_takes_the_max_over_exemplars() {
        let voice = [1.0, 0.0];
        let exemplars = [[0.0, 1.0], [1.0, 0.1]];
        let best = best_score(&voice, exemplars.iter().map(<[f32; 2]>::as_slice));
        assert!(best.unwrap() > 0.99);
        assert_eq!(best_score(&voice, std::iter::empty()), None);
    }

    #[test]
    fn cosine_ignores_scale_and_rejects_mismatches() {
        assert!((cosine(&[1.0, 0.0], &[5.0, 0.0]).unwrap() - 1.0).abs() < 1e-6);
        assert_eq!(cosine(&[1.0, 0.0], &[0.0, 1.0]), Some(0.0));
        assert_eq!(cosine(&[1.0], &[1.0, 0.0]), None);
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 0.0]), None);
        assert_eq!(cosine(&[], &[]), None);
    }

    #[test]
    fn elimination_needs_exact_counts_and_one_open_pair() {
        assert_eq!(
            complete_by_elimination(&[0, 1], &['a', 'b'], &[(0, 'a')]),
            Some((1, 'b'))
        );
        assert_eq!(
            complete_by_elimination(&[0, 1, 2], &['a', 'b'], &[(0, 'a')]),
            None
        );
        assert_eq!(
            complete_by_elimination(&[0, 1, 2], &['a', 'b', 'c'], &[(0, 'a')]),
            None
        );
        assert_eq!(complete_by_elimination::<u8, char>(&[], &[], &[]), None);
    }
}
