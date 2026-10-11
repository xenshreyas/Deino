//! Per-lane voice activity: when someone on a lane is speaking.
//!
//! Each lane runs its own detector on its own audio, the Me lane on the
//! *echo-cancelled* microphone (muesli), so the call's audio leaking into the
//! microphone does not count as the user speaking. The spans drive
//! segmentation for utterance engines, the bleed holdback's "the call was
//! speaking" test, and recording health; the streaming engine itself always
//! gets continuous audio.
//!
//! [`earshot`] scores 16 ms frames; [`VoiceGate`] adds hysteresis: speech
//! starts after [`ONSET_FRAMES`] frames above [`START_SCORE`] and ends after
//! [`HANGOVER`] below [`STOP_SCORE`] (anarlog's live VAD: earshot plus a
//! hangover).

use std::time::Duration;

use deino_stt::Millis;
use earshot::Detector;

/// Samples per detector frame: 16 ms at 16 kHz.
pub const VAD_FRAME: usize = 256;

/// A frame scoring at least this starts speech.
pub const START_SCORE: f32 = 0.5;

/// Speech continues while frames score at least this.
pub const STOP_SCORE: f32 = 0.35;

/// Consecutive speech frames needed to start a span (32 ms).
pub const ONSET_FRAMES: u32 = 2;

/// Silence after which a span ends.
pub const HANGOVER: Duration = Duration::from_millis(600);

/// One stretch of speech on a lane's timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceSpan {
    /// When speech started.
    pub start: Millis,
    /// When it ended (the last voiced frame's end).
    pub end: Millis,
}

/// What one frame changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceChange {
    /// Speech started at this time.
    Started(Millis),
    /// A span of speech ended.
    Ended(VoiceSpan),
}

/// Hysteresis over per-frame speech scores.
#[derive(Clone, Copy, Debug, Default)]
pub struct VoiceGate {
    onset: u32,
    started_at: Option<Millis>,
    last_voiced_end: Millis,
}

impl VoiceGate {
    /// Feeds the score of the frame `[at, at + 16 ms)`.
    pub fn observe(&mut self, score: f32, at: Millis) -> Option<VoiceChange> {
        let end = at + Millis(16);
        let Some(started) = self.started_at else {
            self.onset = if score >= START_SCORE {
                self.onset + 1
            } else {
                0
            };
            if self.onset < ONSET_FRAMES {
                return None;
            }
            let start = at.saturating_sub(Millis(16 * u64::from(ONSET_FRAMES - 1)));
            self.started_at = Some(start);
            self.last_voiced_end = end;
            return Some(VoiceChange::Started(start));
        };
        if score >= STOP_SCORE {
            self.last_voiced_end = end;
            return None;
        }
        let silent = end.saturating_sub(self.last_voiced_end);
        if u128::from(silent.0) < HANGOVER.as_millis() {
            return None;
        }
        Some(self.close(started))
    }

    /// Ends an open span (the lane is finishing).
    pub fn flush(&mut self) -> Option<VoiceChange> {
        let started = self.started_at?;
        Some(self.close(started))
    }

    /// Whether speech is in progress.
    pub fn is_speaking(&self) -> bool {
        self.started_at.is_some()
    }

    fn close(&mut self, started: Millis) -> VoiceChange {
        self.started_at = None;
        self.onset = 0;
        VoiceChange::Ended(VoiceSpan {
            start: started,
            end: self.last_voiced_end,
        })
    }
}

/// A lane's detector: buffers 16 kHz audio into frames and scores them.
pub struct LaneVad {
    detector: Box<Detector>,
    gate: VoiceGate,
    frame: Vec<f32>,
    position: u64,
}

impl std::fmt::Debug for LaneVad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaneVad")
            .field("gate", &self.gate)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

impl Default for LaneVad {
    fn default() -> Self {
        Self {
            detector: Detector::default_boxed(),
            gate: VoiceGate::default(),
            frame: Vec::with_capacity(VAD_FRAME),
            position: 0,
        }
    }
}

impl LaneVad {
    /// Scores `samples` (continuing the lane's audio); calls `changed` for
    /// every start and end of speech.
    pub fn push(&mut self, samples: &[f32], mut changed: impl FnMut(VoiceChange)) {
        for sample in samples {
            self.frame.push(sample.clamp(-1.0, 1.0));
            if self.frame.len() < VAD_FRAME {
                continue;
            }
            let score = self.detector.predict_f32(&self.frame);
            let at = Millis::from_samples(usize::try_from(self.position).unwrap_or(usize::MAX));
            self.position += VAD_FRAME as u64;
            self.frame.clear();
            if let Some(change) = self.gate.observe(score, at) {
                changed(change);
            }
        }
    }

    /// Ends an open span at the end of the recording.
    pub fn finish(&mut self) -> Option<VoiceChange> {
        self.gate.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(gate: &mut VoiceGate, scores: &[f32]) -> Vec<VoiceChange> {
        scores
            .iter()
            .enumerate()
            .filter_map(|(index, score)| gate.observe(*score, Millis(16 * index as u64)))
            .collect()
    }

    #[test]
    fn a_single_loud_frame_is_not_speech() {
        let mut gate = VoiceGate::default();
        assert!(run(&mut gate, &[0.9, 0.1, 0.9, 0.1]).is_empty());
    }

    #[test]
    fn speech_starts_at_its_first_frame_and_ends_after_the_hangover() {
        let mut gate = VoiceGate::default();
        let mut scores = vec![0.0, 0.8, 0.9, 0.6, 0.4];
        scores.extend([0.1; 40]);
        let changes = run(&mut gate, &scores);
        assert_eq!(
            changes,
            [
                VoiceChange::Started(Millis(16)),
                VoiceChange::Ended(VoiceSpan {
                    start: Millis(16),
                    end: Millis(80)
                })
            ]
        );
    }

    #[test]
    fn a_short_pause_does_not_split_speech() {
        let mut gate = VoiceGate::default();
        let mut scores = vec![0.9; 5];
        scores.extend([0.1; 20]);
        scores.extend([0.9; 5]);
        let changes = run(&mut gate, &scores);
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert!(gate.is_speaking());
        assert!(matches!(gate.flush(), Some(VoiceChange::Ended(_))));
    }

    #[test]
    fn silence_never_starts_speech_on_a_lane() {
        let mut vad = LaneVad::default();
        let mut changes = Vec::new();
        vad.push(&vec![0.0; 16_000], |change| changes.push(change));
        assert!(changes.is_empty());
        assert_eq!(vad.finish(), None);
    }
}
