//! What the capture side shares with its supervisor: per-track activity
//! counters and the paused-time ledger. Atomics only, so the conditioner
//! never waits on the supervisor and vice versa.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use crate::units::{Frames, HostTime};

/// One track's capture activity, written by its conditioner.
#[derive(Debug, Default)]
pub struct TrackMeter {
    frames: AtomicU64,
    last_at: AtomicU64,
    peak: AtomicU32,
    /// The loudest sample since the level was last taken (max-hold).
    level: AtomicU32,
}

impl TrackMeter {
    /// Records a delivered block of `len` native-rate frames captured at
    /// `at` whose loudest sample was `peak`.
    pub fn record(&self, len: usize, at: HostTime, peak: f32) {
        let len = u64::try_from(len).unwrap_or(u64::MAX);
        self.frames.fetch_add(len, Ordering::Relaxed);
        self.last_at.store(at.0, Ordering::Relaxed);
        self.peak.store(peak.to_bits(), Ordering::Relaxed);
        // Non-negative floats order like their bit patterns, so the
        // integer max is the float max.
        self.level
            .fetch_max(peak.max(0.0).to_bits(), Ordering::Relaxed);
    }

    /// The loudest sample since the previous call, in `[0, 1]`, and resets
    /// it: a level meter polling at its frame rate sees every peak once.
    /// Meant for one reader (the record panel).
    pub fn take_level(&self) -> f32 {
        f32::from_bits(self.level.swap(0, Ordering::Relaxed))
    }

    /// Native-rate frames delivered so far; the watchdog looks for change.
    pub fn frames(&self) -> Frames {
        Frames(self.frames.load(Ordering::Relaxed))
    }

    /// When the last block was captured.
    pub fn last_at(&self) -> HostTime {
        HostTime(self.last_at.load(Ordering::Relaxed))
    }

    /// The loudest sample of the last block, in `[0, 1]`.
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.peak.load(Ordering::Relaxed))
    }
}

/// Time the recording has spent paused, which the timeline skips.
///
/// While paused the capture sources are stopped; on resume the pause's
/// length is added here, and the conditioners subtract the total from every
/// stamp, so the tracks continue without a gap and the sample index stays
/// recording time.
#[derive(Debug, Default)]
pub struct PauseLedger {
    total_nanos: AtomicU64,
}

impl PauseLedger {
    /// Adds one pause of `length`.
    pub fn add(&self, length: Duration) {
        let nanos = u64::try_from(length.as_nanos()).unwrap_or(u64::MAX);
        self.total_nanos.fetch_add(nanos, Ordering::AcqRel);
    }

    /// Everything paused so far.
    pub fn total(&self) -> Duration {
        Duration::from_nanos(self.total_nanos.load(Ordering::Acquire))
    }

    /// A capture host time moved onto the recording's (pause-free) clock.
    pub fn to_timeline(&self, at: HostTime) -> HostTime {
        HostTime(at.0.saturating_sub(self.total_nanos.load(Ordering::Acquire)))
    }
}

/// The loudest absolute sample of `samples`.
pub fn peak_of(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_meter_accumulates_frames_and_keeps_the_latest_block() {
        let meter = TrackMeter::default();
        meter.record(480, HostTime(10), 0.5);
        meter.record(480, HostTime(20), 0.25);
        assert_eq!(meter.frames(), Frames(960));
        assert_eq!(meter.last_at(), HostTime(20));
        assert!((meter.peak() - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn the_level_holds_the_loudest_peak_until_it_is_taken() {
        let meter = TrackMeter::default();
        meter.record(480, HostTime(10), 0.25);
        meter.record(480, HostTime(20), 0.75);
        meter.record(480, HostTime(30), 0.5);
        assert!((meter.take_level() - 0.75).abs() < f32::EPSILON);
        assert!(meter.take_level().abs() < f32::EPSILON);
    }

    #[test]
    fn paused_time_is_taken_off_capture_stamps() {
        let ledger = PauseLedger::default();
        ledger.add(Duration::from_secs(2));
        ledger.add(Duration::from_secs(1));
        assert_eq!(ledger.total(), Duration::from_secs(3));
        assert_eq!(
            ledger.to_timeline(HostTime(5_000_000_000)),
            HostTime(2_000_000_000)
        );
    }

    #[test]
    fn the_peak_is_the_loudest_absolute_sample() {
        assert!((peak_of(&[0.1, -0.7, 0.3]) - 0.7).abs() < f32::EPSILON);
        assert!(peak_of(&[]).abs() < f32::EPSILON);
    }
}
