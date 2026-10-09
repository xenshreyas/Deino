//! Kaldi log-mel filterbank features, as the WeSpeaker speaker-embedding model
//! expects them.
//!
//! Vendored from knf-rs 0.3.2 (MIT, `thewh1teagle/pyannote-rs@e23bd29`), which
//! wraps kaldi-native-fbank 1.20.1 (Apache-2.0). Deino's changes: the build
//! compiles the C++ with `cc` instead of CMake and bindgen, and this wrapper
//! returns a typed [`Fbank`] and [`FbankError`] instead of `ndarray` and
//! `eyre`. The numbers are unchanged: 16 kHz input, 25 ms Povey windows every
//! 10 ms, 80 mel bins, no dither, `snip_edges`, then per-utterance cepstral
//! mean normalisation (each bin's mean over the frames is subtracted), which
//! is the normalisation WeSpeaker was trained with.

/// The sample rate the extractor is configured for (`knfc/knfc.cc`).
pub const SAMPLE_RATE: u32 = 16_000;

/// Mel bins per frame.
pub const MEL_BINS: usize = 80;

/// Why features could not be computed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FbankError {
    /// No samples were given.
    #[error("no samples to compute features from")]
    Empty,
    /// Fewer samples than one 25 ms frame.
    #[error("the audio is shorter than one feature frame")]
    TooShort,
    /// More samples than the C API can address.
    #[error("the audio is too long for the feature extractor")]
    TooLong,
}

/// Mean-normalised filterbank features: `frames` rows of [`MEL_BINS`] values.
#[derive(Clone, Debug, PartialEq)]
pub struct Fbank {
    values: Vec<f32>,
}

impl Fbank {
    /// Number of 10 ms frames.
    pub fn frames(&self) -> usize {
        self.values.len() / MEL_BINS
    }

    /// Frame `index`, or `None` past the end.
    pub fn frame(&self, index: usize) -> Option<&[f32]> {
        self.values.chunks_exact(MEL_BINS).nth(index)
    }

    /// Every frame, in order.
    pub fn rows(&self) -> std::slice::ChunksExact<'_, f32> {
        self.values.chunks_exact(MEL_BINS)
    }

    /// The frames as one row-major slice.
    pub fn as_slice(&self) -> &[f32] {
        &self.values
    }
}

/// Computes mean-normalised filterbank features of 16 kHz `samples`.
///
/// The model expects 16-bit amplitudes, so callers scale `[-1, 1]` audio by
/// 32768 first (as pyannote's WeSpeaker wrapper does).
pub fn compute_fbank(samples: &[f32]) -> Result<Fbank, FbankError> {
    if samples.is_empty() {
        return Err(FbankError::Empty);
    }
    let mut values = ffi::compute(samples)?;
    if values.is_empty() {
        return Err(FbankError::TooShort);
    }
    subtract_bin_means(&mut values);
    Ok(Fbank { values })
}

/// Cepstral mean normalisation: subtracts each bin's mean over all frames.
fn subtract_bin_means(values: &mut [f32]) {
    let frames = values.len() / MEL_BINS;
    let mut means = [0.0_f32; MEL_BINS];
    for row in values.chunks_exact(MEL_BINS) {
        for (mean, value) in means.iter_mut().zip(row) {
            *mean += value;
        }
    }
    // Frame counts of real audio are far below f32's exact-integer range.
    #[allow(clippy::cast_precision_loss, reason = "frame counts are small")]
    let count = frames as f32;
    for mean in &mut means {
        *mean /= count;
    }
    for row in values.chunks_exact_mut(MEL_BINS) {
        for (value, mean) in row.iter_mut().zip(&means) {
            *value -= mean;
        }
    }
}

#[allow(
    unsafe_code,
    reason = "the two C functions of the knfc shim over kaldi-native-fbank"
)]
mod ffi {
    use super::{FbankError, MEL_BINS};

    /// `FbankResult` in `knfc/knfc.h`.
    #[repr(C)]
    struct FbankResult {
        frames: *mut f32,
        num_frames: i32,
        num_bins: i32,
    }

    unsafe extern "C" {
        fn ComputeFbank(waveform: *const f32, waveform_size: i32) -> FbankResult;
        fn DestroyFbankResult(result: *mut FbankResult);
    }

    /// Runs the extractor and copies its frames out of the C++ buffer.
    pub(super) fn compute(samples: &[f32]) -> Result<Vec<f32>, FbankError> {
        let len = i32::try_from(samples.len()).map_err(|_| FbankError::TooLong)?;
        // SAFETY: `samples` is a live slice of `len` floats; the shim only reads it.
        let mut result = unsafe { ComputeFbank(samples.as_ptr(), len) };
        let frames = usize::try_from(result.num_frames).unwrap_or_default();
        let bins = usize::try_from(result.num_bins).unwrap_or_default();
        let values = if result.frames.is_null() || bins != MEL_BINS {
            Vec::new()
        } else {
            // SAFETY: the shim allocated `num_frames * num_bins` floats at `frames`.
            unsafe { std::slice::from_raw_parts(result.frames, frames * bins) }.to_vec()
        };
        // SAFETY: `result` came from `ComputeFbank` and is destroyed exactly once.
        unsafe { DestroyFbankResult(&raw mut result) };
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "short test tones: small, positive sample counts"
    )]
    fn sine(seconds: f32, frequency: f32) -> Vec<f32> {
        let rate = SAMPLE_RATE as f32;
        (0..(seconds * rate) as usize)
            .map(|i| 16_384.0 * (std::f32::consts::TAU * frequency * i as f32 / rate).sin())
            .collect()
    }

    #[test]
    fn one_second_yields_ninety_eight_frames_of_eighty_bins() {
        // snip_edges: 1 + (16000 - 400) / 160 = 98 frames.
        let features = compute_fbank(&sine(1.0, 440.0)).unwrap();
        assert_eq!(features.frames(), 98);
        assert_eq!(features.rows().len(), 98);
        assert_eq!(features.frame(0).unwrap().len(), MEL_BINS);
        assert!(features.frame(98).is_none());
    }

    #[test]
    fn every_bin_is_mean_normalised() {
        let features = compute_fbank(&sine(0.5, 1_000.0)).unwrap();
        for bin in 0..MEL_BINS {
            let mean: f32 = features.rows().map(|row| row[bin]).sum::<f32>() / 48.0;
            assert!(mean.abs() < 1e-3, "bin {bin} mean {mean}");
        }
    }

    #[test]
    fn a_tone_is_loudest_in_its_own_mel_bin() {
        let low = compute_fbank(&[sine(0.5, 200.0), sine(0.5, 4_000.0)].concat()).unwrap();
        let first = low.frame(5).unwrap();
        let loudest = (0..MEL_BINS)
            .max_by(|a, b| first[*a].total_cmp(&first[*b]))
            .unwrap();
        assert!(
            loudest < 20,
            "a 200 Hz tone peaks in a low bin, got {loudest}"
        );
    }

    #[test]
    fn empty_and_too_short_audio_are_errors() {
        assert_eq!(compute_fbank(&[]), Err(FbankError::Empty));
        assert_eq!(compute_fbank(&[0.0; 100]), Err(FbankError::TooShort));
    }
}
