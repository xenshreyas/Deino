//! `TrackTimeline`: places host-stamped blocks on the shared recording timeline.
//!
//! **The sample index is time.** Every track starts at the recording epoch
//! (`t0`, taken just before either source starts), and frame *n* of a track
//! means `t0 + n / 48 000 s`. A track's position is its cumulative frame
//! count, so presentation times are derived from the running total and never
//! drift. Each block's host stamp is compared with that position:
//!
//! - within ±[`PLACEMENT_TOLERANCE`]: trust the sample count;
//! - later than expected (a source restart, sleep, ring drops): fill the gap
//!   with silence first;
//! - earlier than expected: drop the overlapping head of the block.
//!
//! A track that starts late (the system tap usually does) is therefore
//! **anchored to the epoch** with leading silence, so `mic.m4a` and
//! `system.m4a` line up at t = 0. These rules are the Cap pattern
//! (`docs/references/audio-cap-zed.md` §1–2), reimplemented here.

use std::time::Duration;

use crate::units::{Frames, HostTime, Millis};

/// How far a block may sit from its expected position before it is treated
/// as a gap or an overlap (`audio-capture.md` §4: ±20 ms).
pub const PLACEMENT_TOLERANCE: Duration = Duration::from_millis(20);

/// Where a block lands relative to the frames already on the timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// The block follows the previous one; append it as is.
    Contiguous,
    /// The block starts later than expected: append this much silence first.
    Gap(Frames),
    /// The block starts earlier than expected: drop this many frames from its
    /// head (all of it when the overlap covers the whole block).
    Overlap(Frames),
}

/// One track's position on the recording timeline.
#[derive(Clone, Copy, Debug)]
pub struct TrackTimeline {
    epoch: HostTime,
    /// The timeline frame captured at `epoch`: zero for a new recording,
    /// the recorded length for one that resumes.
    base: Frames,
    written: Frames,
    tolerance: Frames,
}

impl TrackTimeline {
    /// A timeline anchored at the recording epoch `t0`, with nothing written.
    pub fn new(epoch: HostTime) -> Self {
        Self::resuming(epoch, Frames::ZERO)
    }

    /// A timeline that continues a recording `base` frames long: the frame
    /// captured at `epoch` is frame `base`, so a resumed take goes on where
    /// the earlier ones ended (CAP-07), however long ago that was.
    pub fn resuming(epoch: HostTime, base: Frames) -> Self {
        Self {
            epoch,
            base,
            written: base,
            tolerance: Frames::from_duration(PLACEMENT_TOLERANCE),
        }
    }

    /// The recording epoch every track is anchored to.
    pub const fn epoch(&self) -> HostTime {
        self.epoch
    }

    /// Frames on the timeline so far, silence included.
    pub const fn written(&self) -> Frames {
        self.written
    }

    /// The presentation time of the next frame.
    pub fn pts(&self) -> Millis {
        self.written.as_millis()
    }

    /// Places a block of `len` frames whose first frame was captured at
    /// `stamp`, and advances the timeline by what will actually be written.
    pub fn place(&mut self, stamp: HostTime, len: Frames) -> Placement {
        let expected = self.frame_at(stamp);
        if expected > self.written + self.tolerance {
            let gap = expected - self.written;
            self.written += gap + len;
            return Placement::Gap(gap);
        }
        if expected + self.tolerance < self.written {
            let overlap = (self.written - expected).min(len);
            self.written += len - overlap;
            return Placement::Overlap(overlap);
        }
        self.written += len;
        Placement::Contiguous
    }

    /// The silence needed to extend the track up to `end` (the stop time), so
    /// both tracks end together. Advances the timeline by that much.
    pub fn pad_to(&mut self, end: HostTime) -> Frames {
        let tail = self.frame_at(end) - self.written;
        self.written += tail;
        tail
    }

    /// The timeline frame captured at `stamp` (`base` before the epoch).
    fn frame_at(&self, stamp: HostTime) -> Frames {
        self.base + Frames::from_duration(stamp.since(self.epoch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{FakeHostClock, HostClock};

    /// 10 ms blocks, the size a real capture callback delivers.
    const BLOCK: Duration = Duration::from_millis(10);

    fn block() -> Frames {
        Frames::from_duration(BLOCK)
    }

    /// Captures `n` back-to-back blocks, advancing the clock like a source would.
    fn capture(clock: &FakeHostClock, timeline: &mut TrackTimeline, n: usize) -> Vec<Placement> {
        (0..n)
            .map(|_| {
                let placement = timeline.place(clock.now(), block());
                clock.advance(BLOCK);
                placement
            })
            .collect()
    }

    #[test]
    fn a_resumed_timeline_continues_after_the_recorded_length() {
        let clock = FakeHostClock::new(HostTime(7_000));
        let mut timeline = TrackTimeline::resuming(clock.now(), Frames(96_000));

        assert_eq!(
            capture(&clock, &mut timeline, 2),
            [Placement::Contiguous; 2]
        );
        assert_eq!(timeline.written(), Frames(96_960));
        clock.advance(Duration::from_secs(1));
        assert_eq!(timeline.pad_to(clock.now()), Frames(48_000));
    }

    #[test]
    fn back_to_back_blocks_are_contiguous_and_pts_follows_the_sample_count() {
        let clock = FakeHostClock::new(HostTime(7_000));
        let mut timeline = TrackTimeline::new(clock.now());

        let placements = capture(&clock, &mut timeline, 100);

        assert!(placements.iter().all(|p| *p == Placement::Contiguous));
        assert_eq!(timeline.written(), Frames(48_000));
        assert_eq!(timeline.pts(), Millis(1_000));
    }

    #[test]
    fn jitter_within_the_tolerance_keeps_the_sample_count() {
        let clock = FakeHostClock::new(HostTime::default());
        let mut timeline = TrackTimeline::new(clock.now());
        capture(&clock, &mut timeline, 10);

        clock.advance(Duration::from_millis(15));
        let late = timeline.place(clock.now(), block());

        assert_eq!(late, Placement::Contiguous);
        assert_eq!(timeline.written(), Frames(11 * 480));
    }

    #[test]
    fn a_late_block_is_preceded_by_silence_for_the_whole_gap() {
        let clock = FakeHostClock::new(HostTime::default());
        let mut timeline = TrackTimeline::new(clock.now());
        capture(&clock, &mut timeline, 10);

        clock.advance(Duration::from_millis(500));
        let placement = timeline.place(clock.now(), block());

        assert_eq!(placement, Placement::Gap(Frames(24_000)));
        assert_eq!(timeline.written(), Frames(10 * 480 + 24_000 + 480));
    }

    #[test]
    fn an_early_block_loses_its_overlapping_head() {
        let clock = FakeHostClock::new(HostTime(1_000_000_000));
        let mut timeline = TrackTimeline::new(clock.now());
        capture(&clock, &mut timeline, 10);

        let rewound = HostTime(clock.now().0 - 50_000_000);
        let placement = timeline.place(rewound, Frames(4_800));

        assert_eq!(placement, Placement::Overlap(Frames(2_400)));
        assert_eq!(timeline.written(), Frames(10 * 480 + 2_400));
    }

    #[test]
    fn a_block_entirely_inside_the_past_is_dropped() {
        let clock = FakeHostClock::new(HostTime::default());
        let mut timeline = TrackTimeline::new(clock.now());
        capture(&clock, &mut timeline, 100);

        let placement = timeline.place(HostTime::default(), block());

        assert_eq!(placement, Placement::Overlap(block()));
        assert_eq!(timeline.written(), Frames(48_000));
    }

    #[test]
    fn a_track_that_starts_late_is_anchored_to_the_epoch_with_silence() {
        let clock = FakeHostClock::new(HostTime::default());
        let epoch = clock.now();
        let mut microphone = TrackTimeline::new(epoch);
        let mut system = TrackTimeline::new(epoch);

        let mic_first = microphone.place(clock.now(), block());
        clock.advance(Duration::from_millis(750));
        let system_first = system.place(clock.now(), block());

        assert_eq!(mic_first, Placement::Contiguous);
        assert_eq!(system_first, Placement::Gap(Frames(36_000)));
    }

    #[test]
    fn padding_to_the_stop_time_ends_both_tracks_together() {
        let clock = FakeHostClock::new(HostTime::default());
        let epoch = clock.now();
        let mut microphone = TrackTimeline::new(epoch);
        let mut system = TrackTimeline::new(epoch);
        system.place(epoch, Frames(48_000));
        capture(&clock, &mut microphone, 200);
        let stop = clock.now();

        assert_eq!(microphone.pad_to(stop), Frames::ZERO);
        assert_eq!(system.pad_to(stop), Frames(48_000));
        assert_eq!(microphone.written(), system.written());
    }
}
