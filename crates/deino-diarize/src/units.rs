//! The diarizer's sample and frame grid, and the conversions between them.
//!
//! Audio is 16 kHz mono. The segmentation model reads 10 s windows and emits
//! one frame of speaker activity every 270 samples (16.875 ms), the first one
//! centred 721 samples in (its receptive field). Every conversion between
//! samples, frames, seconds and [`Millis`] lives here, so the numeric casts
//! are in one reviewed place.

use std::time::Duration;

use deino_domain::Millis;

/// Samples per second of every diarizer input.
pub const SAMPLE_RATE: u32 = knf_rs::SAMPLE_RATE;

/// The segmentation model's window: exactly 10 s of samples.
pub const WINDOW_SAMPLES: usize = 160_000;

/// Samples summarised by one segmentation output frame.
pub const FRAME_SIZE: usize = 270;

/// Receptive-field offset of the first segmentation output frame.
pub const FRAME_START: usize = 721;

/// Samples per millisecond.
const SAMPLES_PER_MILLI: usize = 16;

/// The first sample of segmentation frame `frame`.
pub const fn frame_start_sample(frame: usize) -> usize {
    FRAME_START + frame * FRAME_SIZE
}

/// Number of segmentation frames fully covered by `samples` input samples.
pub const fn frames_for_samples(samples: usize) -> usize {
    samples.saturating_sub(FRAME_START) / FRAME_SIZE
}

/// The segmentation frame whose span holds sample `sample` of a window.
/// Frame 0 also owns the receptive-field lead-in before it.
pub const fn frame_of_sample(sample: usize) -> usize {
    if sample < FRAME_START + FRAME_SIZE {
        0
    } else {
        (sample - FRAME_START) / FRAME_SIZE
    }
}

/// `duration` in whole segmentation frames, rounded to the nearest.
pub fn duration_to_frames(duration: Duration) -> usize {
    let samples = duration.as_secs_f64() * f64::from(SAMPLE_RATE);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a non-negative duration of at most hours, rounded"
    )]
    let frames = (samples / as_f64(FRAME_SIZE)).round() as usize;
    frames
}

/// `samples` as a [`Duration`].
pub fn samples_to_duration(samples: usize) -> Duration {
    Duration::from_secs_f64(as_f64(samples) / f64::from(SAMPLE_RATE))
}

/// `samples` as [`Millis`], saturating past `u32::MAX` ms (49 days).
pub fn samples_to_millis(samples: usize) -> Millis {
    Millis(u32::try_from(samples / SAMPLES_PER_MILLI).unwrap_or(u32::MAX))
}

/// The sample at `millis`.
pub fn millis_to_samples(millis: Millis) -> usize {
    usize::try_from(millis.get()).unwrap_or(usize::MAX) * SAMPLES_PER_MILLI
}

/// `value` as `f32`, exact for every count the diarizer handles (< 2^24).
pub fn as_f32(value: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "frame, window and anchor counts stay far below 2^24"
    )]
    let value = value as f32;
    value
}

/// `value` as `f64`, exact for every sample count below 2^53.
pub fn as_f64(value: usize) -> f64 {
    #[allow(clippy::cast_precision_loss, reason = "sample counts stay below 2^53")]
    let value = value as f64;
    value
}

/// `numerator / denominator` rounded up to a whole count, at least 0.
pub fn ceil_ratio(numerator: f32, denominator: f32) -> usize {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small, non-negative ratio of durations"
    )]
    let count = (numerator / denominator.max(f32::EPSILON)).ceil().max(0.0) as usize;
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_window_holds_five_hundred_eighty_nine_frames() {
        assert_eq!(frames_for_samples(WINDOW_SAMPLES), 589);
        assert_eq!(frames_for_samples(FRAME_START - 1), 0);
    }

    #[test]
    fn samples_map_back_to_the_frame_that_covers_them() {
        assert_eq!(frame_of_sample(0), 0);
        assert_eq!(frame_of_sample(frame_start_sample(1) - 1), 0);
        assert_eq!(frame_of_sample(frame_start_sample(1)), 1);
        assert_eq!(frame_of_sample(frame_start_sample(7) + 5), 7);
    }

    #[test]
    fn durations_round_to_the_nearest_frame() {
        assert_eq!(duration_to_frames(Duration::from_secs(2)), 119);
        assert_eq!(duration_to_frames(Duration::ZERO), 0);
    }

    #[test]
    fn millis_and_samples_convert_both_ways() {
        assert_eq!(samples_to_millis(16_000), Millis(1_000));
        assert_eq!(millis_to_samples(Millis(250)), 4_000);
        assert_eq!(samples_to_duration(8_000), Duration::from_millis(500));
    }

    #[test]
    fn ceil_ratio_rounds_up_and_never_divides_by_zero() {
        assert_eq!(ceil_ratio(12.0, 2.0), 6);
        assert_eq!(ceil_ratio(12.0, 5.0), 3);
        assert!(ceil_ratio(1.0, 0.0) > 0);
    }
}
