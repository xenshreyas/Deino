//! The audio format every engine takes, and time measured on its clock.
//!
//! Engines read 16 kHz mono `f32` PCM in `[-1, 1]`. Positions on a recording's
//! timeline are [`Millis`] since the recording began, so a word's time can be
//! compared with the audio file and with the other lane.

use std::ops::{Add, AddAssign};

use serde::{Deserialize, Serialize};

/// Samples per second of the PCM every engine reads.
pub const SAMPLE_RATE_HZ: u32 = 16_000;

/// Samples per millisecond at [`SAMPLE_RATE_HZ`].
const SAMPLES_PER_MS: u64 = SAMPLE_RATE_HZ as u64 / 1000;

/// A duration or a position on a recording's timeline, in milliseconds.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Millis(pub u64);

impl Millis {
    /// The start of a timeline.
    pub const ZERO: Self = Self(0);

    /// The duration of `samples` of 16 kHz audio, rounded down.
    pub fn from_samples(samples: usize) -> Self {
        Self(u64::try_from(samples).unwrap_or(u64::MAX) / SAMPLES_PER_MS)
    }

    /// A signed engine time (stream-relative, never meaningfully negative),
    /// clamped at zero.
    pub fn from_engine(ms: i64) -> Self {
        Self(u64::try_from(ms).unwrap_or(0))
    }

    /// The number of 16 kHz samples in this duration.
    pub fn to_samples(self) -> usize {
        usize::try_from(self.0.saturating_mul(SAMPLES_PER_MS)).unwrap_or(usize::MAX)
    }

    /// `self - other`, or zero when `other` is later.
    #[must_use]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// This duration in seconds, for logs and reports.
    #[allow(
        clippy::cast_precision_loss,
        reason = "durations stay far below 2^52 ms, where f64 is exact"
    )]
    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 / 1000.0
    }
}

impl Add for Millis {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl AddAssign for Millis {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_counts_convert_at_sixteen_per_millisecond() {
        assert_eq!(Millis::from_samples(16_000), Millis(1000));
        assert_eq!(Millis::from_samples(15), Millis(0));
        assert_eq!(Millis(250).to_samples(), 4000);
    }

    #[test]
    fn engine_times_before_the_stream_start_clamp_to_zero() {
        assert_eq!(Millis::from_engine(-80), Millis::ZERO);
        assert_eq!(Millis::from_engine(560), Millis(560));
    }

    #[test]
    fn arithmetic_saturates_instead_of_wrapping() {
        assert_eq!(Millis(5).saturating_sub(Millis(9)), Millis::ZERO);
        assert_eq!(Millis(u64::MAX) + Millis(1), Millis(u64::MAX));
    }
}
