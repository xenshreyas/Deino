//! Units of the recording timeline: tracks, frames, milliseconds, host time.
//!
//! Deino stores every track as 48 kHz mono, and **the sample index is time**:
//! frame *n* of `mic.m4a` and frame *n* of `system.m4a` both mean
//! `epoch + n / 48 000 s` (`docs/references/audio-capture.md` §4). These
//! newtypes keep frames, milliseconds, nanoseconds and bytes from being mixed up.

use std::fmt;
use std::ops::{Add, AddAssign, Sub};
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// The sample rate of every stored track, in frames per second.
pub const STORAGE_RATE: u32 = 48_000;

/// Nanoseconds in one second.
const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// Milliseconds in one second.
const MILLIS_PER_SECOND: u64 = 1_000;

/// One of the two audio tracks Deino records. They stay separate end to end
/// (§3C); the mixed `audio.mp4` is derived from them at finalize.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Track {
    /// The user's microphone ("Me").
    Microphone,
    /// Everything the Mac plays, from the Core Audio process tap ("Them").
    System,
}

impl Track {
    /// Both tracks, microphone first.
    pub const ALL: [Self; 2] = [Self::Microphone, Self::System];

    /// The file stem used on disk: `mic` or `system` (`mic.001.m4a.part`).
    pub const fn file_stem(self) -> &'static str {
        match self {
            Self::Microphone => "mic",
            Self::System => "system",
        }
    }
}

impl fmt::Display for Track {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.file_stem())
    }
}

/// A count of 48 kHz frames, or a position on the recording timeline.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Frames(pub u64);

impl Frames {
    /// No frames.
    pub const ZERO: Self = Self(0);

    /// The frames covering `duration` at [`STORAGE_RATE`], rounded down.
    pub fn from_duration(duration: Duration) -> Self {
        let frames = duration.as_nanos() * u128::from(STORAGE_RATE) / NANOS_PER_SECOND;
        Self(u64::try_from(frames).unwrap_or(u64::MAX))
    }

    /// The frames in `len` samples of a mono buffer.
    pub fn of_len(len: usize) -> Self {
        Self(u64::try_from(len).unwrap_or(u64::MAX))
    }

    /// The presentation time of this frame index, rounded down to a millisecond.
    pub fn as_millis(self) -> Millis {
        Millis(self.0 * MILLIS_PER_SECOND / u64::from(STORAGE_RATE))
    }

    /// The duration these frames last.
    pub fn as_duration(self) -> Duration {
        let nanos = u128::from(self.0) * NANOS_PER_SECOND / u128::from(STORAGE_RATE);
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }

    /// `self - other`, or zero when `other` is larger.
    #[must_use]
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// The count as a buffer length (saturating on 32-bit targets).
    pub fn as_len(self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX)
    }
}

impl Add for Frames {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl AddAssign for Frames {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sub for Frames {
    type Output = Self;

    /// Saturating: a timeline position never goes below zero.
    fn sub(self, other: Self) -> Self {
        self.saturating_sub(other)
    }
}

/// A presentation time or duration in whole milliseconds.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Millis(pub u64);

/// A reading of the monotonic host clock, in nanoseconds since an arbitrary
/// origin (on macOS, `mach_absolute_time` converted to nanoseconds, the same
/// clock as `AudioTimeStamp::mHostTime`).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct HostTime(pub u64);

impl HostTime {
    /// The time `duration` after `self`.
    #[must_use]
    pub fn plus(self, duration: Duration) -> Self {
        let nanos = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
        Self(self.0.saturating_add(nanos))
    }

    /// The time elapsed from `earlier` to `self`, or zero if `earlier` is later.
    pub fn since(self, earlier: Self) -> Duration {
        Duration::from_nanos(self.0.saturating_sub(earlier.0))
    }
}

/// A size in bytes (file sizes, free disk space, memory budgets).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Bytes(pub u64);

impl Bytes {
    /// `n` mebibytes.
    pub const fn mib(n: u64) -> Self {
        Self(n * 1024 * 1024)
    }

    /// `n` gibibytes.
    pub const fn gib(n: u64) -> Self {
        Self(n * 1024 * 1024 * 1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_second_of_frames_is_one_thousand_millis() {
        let second = Frames::from_duration(Duration::from_secs(1));
        assert_eq!(second, Frames(48_000));
        assert_eq!(second.as_millis(), Millis(1_000));
        assert_eq!(second.as_duration(), Duration::from_secs(1));
    }

    #[test]
    fn frame_arithmetic_never_goes_negative() {
        assert_eq!(Frames(5) - Frames(9), Frames::ZERO);
        assert_eq!(HostTime(5).since(HostTime(9)), Duration::ZERO);
    }

    #[test]
    fn tracks_serialize_by_their_full_name_and_store_by_their_stem() {
        let json = serde_json::to_string(&Track::Microphone).unwrap();
        assert_eq!(json, r#""microphone""#);
        assert_eq!(Track::Microphone.file_stem(), "mic");
        assert_eq!(Track::System.to_string(), "system");
    }
}
