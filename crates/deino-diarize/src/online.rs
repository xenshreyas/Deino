//! Best-effort live speaker labels: online clustering of finished
//! utterances while the meeting runs.
//!
//! Pure logic for CAP-02 to drive from the system lane (embedding happens
//! off the lane thread); the post-meeting diarization replaces its labels.
//! The rules follow OpenWhispr's `liveSpeakerIdentifier.js` (MIT) with the
//! spike's WeSpeaker r34 thresholds (`docs/references/spike-diarization.md`):
//!
//! - only utterances of at least 1.5 s are embedded (shorter ones are too
//!   noisy to place);
//! - an utterance joins a speaker when its best cosine similarity is at
//!   least 0.60 **and** beats the runner-up by 0.05;
//! - only a confident join moves that speaker's centroid (a running mean);
//! - a voice close to two speakers, or arriving at the speaker cap, is
//!   labelled with the nearest speaker but never folded into it;
//! - IDs are never reused.

use std::time::Duration;

use crate::vector::{Embedding, cosine_similarity, normalize};

/// Similarity an utterance needs to join a speaker.
pub const JOIN_SCORE: f32 = 0.60;
/// Lead the best speaker needs over the runner-up.
pub const JOIN_MARGIN: f32 = 0.05;
/// Shorter utterances are not placed.
pub const MIN_UTTERANCE: Duration = Duration::from_millis(1_500);

/// A live speaker, numbered from 0 in order of first appearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LiveSpeakerId(pub u32);

/// How the tracker decides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveConfig {
    /// See [`JOIN_SCORE`].
    pub join_score: f32,
    /// See [`JOIN_MARGIN`].
    pub margin: f32,
    /// See [`MIN_UTTERANCE`].
    pub min_utterance: Duration,
    /// The most speakers to create (the calendar's expected count); `None`
    /// for no cap.
    pub max_speakers: Option<usize>,
}

impl Default for LiveConfig {
    fn default() -> Self {
        Self {
            join_score: JOIN_SCORE,
            margin: JOIN_MARGIN,
            min_utterance: MIN_UTTERANCE,
            max_speakers: None,
        }
    }
}

/// Where an utterance went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LiveMatch {
    /// A confident match; the speaker's centroid moved toward it.
    Joined {
        /// The speaker.
        speaker: LiveSpeakerId,
        /// Its similarity.
        score: f32,
    },
    /// A new voice.
    Created(LiveSpeakerId),
    /// Labelled with the nearest speaker but not folded in: ambiguous
    /// between two speakers, or no match while at the cap.
    Nearest {
        /// The speaker.
        speaker: LiveSpeakerId,
        /// Its similarity.
        score: f32,
    },
    /// Too short to place.
    TooShort,
}

/// One live speaker: the sum of its unit embeddings (the direction of
/// their running mean).
#[derive(Clone, Debug, PartialEq)]
struct Voice {
    id: LiveSpeakerId,
    sum: Vec<f32>,
}

impl Voice {
    fn centroid(&self) -> Vec<f32> {
        normalize(&self.sum)
    }
}

/// Online clustering over one meeting's utterances.
#[derive(Clone, Debug, Default)]
pub struct LiveSpeakerTracker {
    config: LiveConfig,
    voices: Vec<Voice>,
    next: u32,
}

impl LiveSpeakerTracker {
    /// An empty tracker.
    pub fn new(config: LiveConfig) -> Self {
        Self {
            config,
            voices: Vec::new(),
            next: 0,
        }
    }

    /// Places one utterance of length `duration` with `embedding`.
    pub fn observe(&mut self, embedding: &Embedding, duration: Duration) -> LiveMatch {
        if duration < self.config.min_utterance {
            return LiveMatch::TooShort;
        }
        let unit = normalize(embedding.as_slice());
        let ranked = self.ranked(&unit);
        let best = ranked.first().copied();
        let runner_up = ranked.get(1).map_or(f32::NEG_INFINITY, |(_, score)| *score);
        match best {
            Some((index, score)) if score >= self.config.join_score => {
                if score - runner_up >= self.config.margin {
                    self.join(index, &unit, score)
                } else {
                    self.nearest(index, score)
                }
            }
            Some((index, score)) if self.at_cap() => self.nearest(index, score),
            _ => self.create(unit),
        }
    }

    /// Each speaker's centroid (unit length), in creation order.
    pub fn centroids(&self) -> Vec<(LiveSpeakerId, Embedding)> {
        self.voices
            .iter()
            .filter_map(|voice| Some((voice.id, Embedding::new(voice.centroid())?)))
            .collect()
    }

    /// `(voice index, similarity)`, best first.
    fn ranked(&self, unit: &[f32]) -> Vec<(usize, f32)> {
        let mut scores: Vec<(usize, f32)> = self
            .voices
            .iter()
            .enumerate()
            .filter_map(|(index, voice)| Some((index, cosine_similarity(unit, &voice.sum)?)))
            .collect();
        scores.sort_by(|a, b| b.1.total_cmp(&a.1));
        scores
    }

    fn at_cap(&self) -> bool {
        self.config
            .max_speakers
            .is_some_and(|cap| self.voices.len() >= cap)
    }

    fn join(&mut self, index: usize, unit: &[f32], score: f32) -> LiveMatch {
        let Some(voice) = self.voices.get_mut(index) else {
            return LiveMatch::TooShort;
        };
        voice
            .sum
            .iter_mut()
            .zip(unit)
            .for_each(|(sum, value)| *sum += value);
        LiveMatch::Joined {
            speaker: voice.id,
            score,
        }
    }

    fn nearest(&self, index: usize, score: f32) -> LiveMatch {
        self.voices
            .get(index)
            .map_or(LiveMatch::TooShort, |voice| LiveMatch::Nearest {
                speaker: voice.id,
                score,
            })
    }

    fn create(&mut self, unit: Vec<f32>) -> LiveMatch {
        let id = LiveSpeakerId(self.next);
        self.next += 1;
        self.voices.push(Voice { id, sum: unit });
        LiveMatch::Created(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LONG: Duration = Duration::from_secs(3);

    fn voice(values: &[f32]) -> Embedding {
        Embedding::new(values.to_vec()).unwrap()
    }

    #[test]
    fn a_new_voice_creates_a_speaker_and_a_close_one_joins_it() {
        let mut tracker = LiveSpeakerTracker::new(LiveConfig::default());
        assert_eq!(
            tracker.observe(&voice(&[1.0, 0.0, 0.0]), LONG),
            LiveMatch::Created(LiveSpeakerId(0))
        );
        assert!(matches!(
            tracker.observe(&voice(&[0.95, 0.2, 0.0]), LONG),
            LiveMatch::Joined {
                speaker: LiveSpeakerId(0),
                ..
            }
        ));
        assert_eq!(
            tracker.observe(&voice(&[0.0, 1.0, 0.0]), LONG),
            LiveMatch::Created(LiveSpeakerId(1))
        );
    }

    #[test]
    fn short_utterances_are_not_placed() {
        let mut tracker = LiveSpeakerTracker::new(LiveConfig::default());
        let short = Duration::from_millis(1_400);
        assert_eq!(
            tracker.observe(&voice(&[1.0, 0.0]), short),
            LiveMatch::TooShort
        );
        assert!(tracker.centroids().is_empty());
    }

    #[test]
    fn an_ambiguous_voice_is_labelled_but_moves_no_centroid() {
        let mut tracker = LiveSpeakerTracker::new(LiveConfig::default());
        tracker.observe(&voice(&[1.0, 0.0]), LONG);
        tracker.observe(&voice(&[0.5, 0.866]), LONG);
        let before = tracker.centroids();
        assert_eq!(before.len(), 2);
        let result = tracker.observe(&voice(&[0.866, 0.5]), LONG);
        assert!(matches!(result, LiveMatch::Nearest { .. }), "{result:?}");
        assert_eq!(tracker.centroids(), before);
    }

    #[test]
    fn at_the_cap_a_stranger_gets_the_nearest_label() {
        let config = LiveConfig {
            max_speakers: Some(1),
            ..LiveConfig::default()
        };
        let mut tracker = LiveSpeakerTracker::new(config);
        tracker.observe(&voice(&[1.0, 0.0]), LONG);
        let result = tracker.observe(&voice(&[0.0, 1.0]), LONG);
        assert!(matches!(
            result,
            LiveMatch::Nearest {
                speaker: LiveSpeakerId(0),
                ..
            }
        ));
        assert_eq!(tracker.centroids().len(), 1);
    }

    #[test]
    fn ids_are_never_reused() {
        let mut tracker = LiveSpeakerTracker::new(LiveConfig::default());
        let ids: Vec<LiveMatch> = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
            .iter()
            .map(|v| tracker.observe(&voice(v), LONG))
            .collect();
        assert_eq!(
            ids,
            vec![
                LiveMatch::Created(LiveSpeakerId(0)),
                LiveMatch::Created(LiveSpeakerId(1)),
                LiveMatch::Created(LiveSpeakerId(2)),
            ]
        );
    }
}
