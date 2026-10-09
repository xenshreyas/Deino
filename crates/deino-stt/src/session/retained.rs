//! The audio a session keeps in case its stream is lost.
//!
//! Everything fed since the last closed segment (or since the decoder last
//! passed silence) is kept, so a lost stream loses no words: the session
//! transcribes this audio in one run. It is trimmed as segments close, so it
//! stays around one segment long instead of growing with the meeting.

use crate::pcm::Millis;

/// Retained audio and where it starts on the recording's timeline.
#[derive(Debug, Default)]
pub struct Retained {
    /// Timeline position of the first retained sample, in samples.
    start: usize,
    samples: Vec<f32>,
}

impl Retained {
    /// Empty, starting at `position`.
    pub fn at(position: Millis) -> Self {
        Self {
            start: position.to_samples(),
            samples: Vec::new(),
        }
    }

    /// Where the retained audio starts.
    pub fn start(&self) -> Millis {
        Millis::from_samples(self.start)
    }

    /// How much audio is retained.
    pub fn duration(&self) -> Millis {
        Millis::from_samples(self.samples.len())
    }

    /// Appends audio that was just fed.
    pub fn push(&mut self, pcm: &[f32]) {
        self.samples.extend_from_slice(pcm);
    }

    /// Drops the audio before `position` (never past the retained end).
    pub fn trim_before(&mut self, position: Millis) {
        let drop = position
            .to_samples()
            .saturating_sub(self.start)
            .min(self.samples.len());
        if drop > 0 {
            self.samples.drain(..drop);
            self.start += drop;
        }
    }

    /// Takes all retained audio, with its start; what remains starts after it.
    pub fn take(&mut self) -> (Millis, Vec<f32>) {
        let start = self.start();
        let samples = std::mem::take(&mut self.samples);
        self.start += samples.len();
        (start, samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trimming_drops_audio_before_a_position_but_never_past_the_end() {
        let mut retained = Retained::at(Millis(1000));
        retained.push(&vec![0.0; Millis(500).to_samples()]);

        retained.trim_before(Millis(1200));
        assert_eq!(
            (retained.start(), retained.duration()),
            (Millis(1200), Millis(300))
        );

        retained.trim_before(Millis(900));
        assert_eq!(retained.start(), Millis(1200));

        retained.trim_before(Millis(9000));
        assert_eq!(
            (retained.start(), retained.duration()),
            (Millis(1500), Millis::ZERO)
        );
    }

    #[test]
    fn taking_moves_the_start_past_the_taken_audio() {
        let mut retained = Retained::at(Millis::ZERO);
        retained.push(&vec![0.0; Millis(250).to_samples()]);

        let (start, samples) = retained.take();

        assert_eq!((start, samples.len()), (Millis::ZERO, 4000));
        assert_eq!(retained.start(), Millis(250));
    }
}
