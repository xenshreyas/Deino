//! Capture-rate audio to the 16 kHz the engine reads, as a stream.
//!
//! The live pipeline mixes at the device rate (48 kHz); the engine wants
//! 16 kHz. One stateful resampler per lane converts every window exactly
//! once, with no edge artefacts between windows, and drops the resampler's
//! own delay so token times stay on the recording's timeline.

use rubato::{FftFixedInOut, Resampler, ResamplerConstructionError};

use deino_stt::SAMPLE_RATE_HZ;

/// Requested input chunk: 20 ms at 48 kHz (rubato may round it).
const CHUNK_IN: usize = 960;

/// A streaming resampler to 16 kHz mono.
pub struct Downsampler {
    /// `None` when the input is already 16 kHz.
    resampler: Option<FftFixedInOut<f32>>,
    pending: Vec<f32>,
    /// Leading output samples still to drop (the resampler's delay).
    delay: usize,
}

impl Downsampler {
    /// A resampler from `input_rate` Hz.
    pub fn new(input_rate: u32) -> Result<Self, ResamplerConstructionError> {
        if input_rate == SAMPLE_RATE_HZ {
            return Ok(Self {
                resampler: None,
                pending: Vec::new(),
                delay: 0,
            });
        }
        let rates = (input_rate as usize, SAMPLE_RATE_HZ as usize);
        let resampler = FftFixedInOut::new(rates.0, rates.1, CHUNK_IN, 1)?;
        Ok(Self {
            delay: resampler.output_delay(),
            resampler: Some(resampler),
            pending: Vec::new(),
        })
    }

    /// Converts `input`; returns the 16 kHz samples ready so far.
    pub fn push(&mut self, input: &[f32]) -> Vec<f32> {
        let Some(resampler) = self.resampler.as_mut() else {
            return input.to_vec();
        };
        self.pending.extend_from_slice(input);
        let mut output = Vec::new();
        while self.pending.len() >= resampler.input_frames_next() {
            let take = resampler.input_frames_next();
            let chunk: Vec<f32> = self.pending.drain(..take).collect();
            match resampler.process(&[chunk], None) {
                Ok(mut channels) => output.append(&mut channels.swap_remove(0)),
                Err(error) => log::warn!(target: "stt", "dropping a resampler chunk: {error}"),
            }
        }
        let skip = self.delay.min(output.len());
        self.delay -= skip;
        output.drain(..skip);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forty_eight_kilohertz_becomes_a_third_as_many_samples() {
        let mut downsampler = Downsampler::new(48_000).unwrap();
        let second = vec![0.25_f32; 48_000];

        let mut output = downsampler.push(&second);
        output.extend(downsampler.push(&second));

        // Two seconds in, about two seconds out (less the resampler's delay).
        let expected = 2 * SAMPLE_RATE_HZ as usize;
        assert!(
            output.len() <= expected && output.len() + 1000 > expected,
            "{}",
            output.len()
        );
        let middle = output[output.len() / 2];
        assert!((middle - 0.25).abs() < 0.01, "level kept: {middle}");
    }

    #[test]
    fn sixteen_kilohertz_passes_through_untouched() {
        let mut downsampler = Downsampler::new(16_000).unwrap();

        assert_eq!(downsampler.push(&[0.1, 0.2]), [0.1, 0.2]);
    }
}
