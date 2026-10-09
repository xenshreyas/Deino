//! The embedding model's input: filterbank frames of the speech of one
//! speaker.
//!
//! A window's filterbank is computed once and shared by its local speakers;
//! each speaker keeps only the frames where segmentation heard them (anarlog
//! computed the same filterbank once per speaker). Frame selection follows
//! anarlog's nearest-neighbour mask resampling exactly, so the embeddings are
//! unchanged.

use knf_rs::{Fbank, FbankError, MEL_BINS};

use crate::error::DiarizeError;
use crate::units::frame_of_sample;

/// Scale from `[-1, 1]` samples to the 16-bit amplitudes WeSpeaker was
/// trained on (pyannote's `ONNXWeSpeakerPretrainedSpeakerEmbedding`).
const WAVEFORM_SCALE: f32 = 32_768.0;

/// Filterbank frames for one embedding: `frames` rows of [`MEL_BINS`].
#[derive(Clone, Debug, PartialEq)]
pub struct Features {
    values: Vec<f32>,
}

impl Features {
    /// Number of 10 ms frames.
    pub fn frames(&self) -> usize {
        self.values.len() / MEL_BINS
    }

    /// The frames as one row-major slice.
    pub fn as_slice(&self) -> &[f32] {
        &self.values
    }
}

/// The filterbank of one stretch of audio (a window or an utterance).
#[derive(Clone, Debug)]
pub struct AudioFeatures {
    fbank: Fbank,
    samples: usize,
}

impl AudioFeatures {
    /// Computes the filterbank of `samples`; `None` when they are shorter
    /// than one 25 ms frame.
    pub fn compute(samples: &[f32]) -> Result<Option<Self>, DiarizeError> {
        let scaled: Vec<f32> = samples
            .iter()
            .map(|sample| sample * WAVEFORM_SCALE)
            .collect();
        match knf_rs::compute_fbank(&scaled) {
            Ok(fbank) => Ok(Some(Self {
                fbank,
                samples: samples.len(),
            })),
            Err(FbankError::TooShort) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// Every frame.
    pub fn all(&self) -> Features {
        Features {
            values: self.fbank.as_slice().to_vec(),
        }
    }

    /// The frames whose audio falls in a segmentation frame `keep` accepts;
    /// `None` when none do. Filterbank frame `t` of `T` maps to sample
    /// `t * samples / T` and from there to its segmentation frame.
    pub fn masked(&self, keep: impl Fn(usize) -> bool) -> Option<Features> {
        let total = self.fbank.frames();
        let values: Vec<f32> = self
            .fbank
            .rows()
            .enumerate()
            .filter(|(index, _)| keep(frame_of_sample(index * self.samples / total)))
            .flat_map(|(_, row)| row.iter().copied())
            .collect();
        (!values.is_empty()).then_some(Features { values })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{WINDOW_SAMPLES, frame_start_sample};

    fn noise(samples: usize) -> Vec<f32> {
        (0..samples)
            .map(|index| {
                (f32::from(u16::try_from(index * 7_919 % 1_000).unwrap()) / 1_000.0 - 0.5) * 0.2
            })
            .collect()
    }

    #[test]
    fn a_window_has_nine_hundred_ninety_eight_frames() {
        let features = AudioFeatures::compute(&noise(WINDOW_SAMPLES))
            .unwrap()
            .unwrap();
        assert_eq!(features.all().frames(), 998);
    }

    #[test]
    fn audio_shorter_than_one_frame_has_no_features() {
        assert!(AudioFeatures::compute(&noise(100)).unwrap().is_none());
        assert!(AudioFeatures::compute(&[]).is_err());
    }

    #[test]
    fn masking_keeps_the_frames_under_kept_segmentation_frames() {
        let features = AudioFeatures::compute(&noise(WINDOW_SAMPLES))
            .unwrap()
            .unwrap();
        let everything = features.masked(|_| true).unwrap();
        assert_eq!(everything, features.all());
        assert!(features.masked(|_| false).is_none());

        // The first half of the window: segmentation frames before 5 s.
        let half = frame_of_sample(WINDOW_SAMPLES / 2);
        let first_half = features.masked(|frame| frame < half).unwrap();
        assert!(
            (495..=505).contains(&first_half.frames()),
            "{}",
            first_half.frames()
        );
        assert!(frame_start_sample(half) <= WINDOW_SAMPLES / 2 + 270);
    }
}
