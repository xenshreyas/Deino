//! The voice check: DeepFilterNet3 judges whether a short stretch of the
//! microphone track holds a voice (see the README).
//!
//! Levels are measured on 100 ms frames on the track's grid: frame `n`
//! covers `[n × 100 ms, (n + 1) × 100 ms)`. The model cleans from
//! [`PRE_ROLL_FRAMES`] before the frames asked about, so its state has
//! settled on the room's noise before it is judged.

use std::ops::Range;

use df::tract::{DfParams, DfTract, RuntimeParams};
use df::transforms::resample;
use ndarray::{Array2, Axis};
// Pinned in Cargo.toml for deep_filter's sake (see there), not used directly.
use {tract_core as _, tract_hir as _, tract_onnx as _, tract_pulse as _};

/// The sample rate this crate takes.
pub const SAMPLE_RATE_HZ: usize = 16_000;
/// Samples in one 100 ms frame at [`SAMPLE_RATE_HZ`].
pub const FRAME: usize = SAMPLE_RATE_HZ / 10;
/// Frames the model runs before a stretch (3 s).
const PRE_ROLL_FRAMES: usize = 30;
/// The level of digital silence.
const SILENCE_DB: f32 = -180.0;
/// The DeepFilterNet CLI's thresholds (`deep-filter` 0.5 defaults), which
/// the A/B test that chose the rule ran with: no attenuation limit, local
/// SNR thresholds -15, 35 and 35 dB.
const ATTENUATION_LIMIT_DB: f32 = 100.0;
const MIN_DB: f32 = -15.0;
const MAX_ERB_DB: f32 = 35.0;
const MAX_DF_DB: f32 = 35.0;

/// Why the model could not run.
#[derive(Debug, thiserror::Error)]
pub enum DenoiseError {
    /// Loading or running the model failed.
    #[error("the voice check model failed: {0}")]
    Model(String),
}

/// One loaded DeepFilterNet3 model.
pub struct Cleaner {
    model: DfTract,
}

impl std::fmt::Debug for Cleaner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cleaner").finish_non_exhaustive()
    }
}

impl Cleaner {
    /// Loads the embedded model (a fraction of a second).
    pub fn new() -> Result<Self, DenoiseError> {
        let params = RuntimeParams::default_with_ch(1)
            .with_atten_lim(ATTENUATION_LIMIT_DB)
            .with_thresholds(MIN_DB, MAX_ERB_DB, MAX_DF_DB);
        let model = DfTract::new(DfParams::default(), &params).map_err(model_error)?;
        Ok(Self { model })
    }

    /// The loudest 100 ms frame of the cleaned audio over `frames` of
    /// 16 kHz `pcm`, in dBFS; silence when the frames are past the end.
    pub fn cleaned_peak(&mut self, pcm: &[f32], frames: Range<usize>) -> Result<f32, DenoiseError> {
        let frames = at_least_one(frames);
        let first = frames.start.saturating_sub(PRE_ROLL_FRAMES);
        let end = frames.end.saturating_mul(FRAME).min(pcm.len());
        let Some(stretch) = pcm.get(first * FRAME..end).filter(|s| !s.is_empty()) else {
            return Ok(SILENCE_DB);
        };
        let cleaned = self.clean(stretch)?;
        let frame = FRAME * self.model.sr / SAMPLE_RATE_HZ;
        let offset = frames.start - first;
        Ok(peak_of(&cleaned, frame, offset..offset + frames.len()))
    }

    /// `stretch` cleaned, at the model's rate.
    fn clean(&mut self, stretch: &[f32]) -> Result<Vec<f32>, DenoiseError> {
        let noisy =
            Array2::from_shape_vec((1, stretch.len()), stretch.to_vec()).map_err(model_error)?;
        let noisy =
            resample(noisy.view(), SAMPLE_RATE_HZ, self.model.sr, None).map_err(model_error)?;
        let mut cleaned = Array2::<f32>::zeros(noisy.raw_dim());
        let hop = self.model.hop_size;
        let hops = noisy
            .axis_chunks_iter(Axis(1), hop)
            .zip(cleaned.axis_chunks_iter_mut(Axis(1), hop));
        for (input, output) in hops {
            if input.len_of(Axis(1)) < hop {
                break;
            }
            self.model.process(input, output).map_err(model_error)?;
        }
        Ok(cleaned.into_raw_vec())
    }
}

/// The loudest 100 ms frame of 16 kHz `pcm` over `frames`, in dBFS.
pub fn peak_db(pcm: &[f32], frames: Range<usize>) -> f32 {
    peak_of(pcm, FRAME, at_least_one(frames))
}

/// The 100 ms frames a span of `start_ms..end_ms` is judged on: from the
/// one it starts in, to the last one it fully covers (at least one).
pub fn frames_of(start_ms: u64, end_ms: u64) -> Range<usize> {
    let to_frame = |ms: u64| usize::try_from(ms / 100).unwrap_or(usize::MAX);
    at_least_one(to_frame(start_ms)..to_frame(end_ms))
}

fn at_least_one(frames: Range<usize>) -> Range<usize> {
    frames.start..frames.end.max(frames.start.saturating_add(1))
}

fn peak_of(samples: &[f32], frame: usize, frames: Range<usize>) -> f32 {
    frames
        .filter_map(|at| samples.get(at * frame..(at + 1) * frame))
        .map(level_db)
        .fold(SILENCE_DB, f32::max)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a frame is at most a few thousand samples"
)]
fn level_db(frame: &[f32]) -> f32 {
    let energy = frame.iter().map(|sample| sample * sample).sum::<f32>() / frame.len() as f32;
    20.0 * (energy.sqrt() + 1e-9).log10()
}

fn model_error(error: impl std::fmt::Display) -> DenoiseError {
    DenoiseError::Model(error.to_string())
}

#[cfg(test)]
mod tests;
