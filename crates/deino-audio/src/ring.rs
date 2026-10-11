//! The real-time boundary: preallocated SPSC rings of stamped mono blocks.
//!
//! A capture callback runs on a Core Audio real-time thread, where it must not
//! allocate, lock, log or make system calls (§3C). It only touches a
//! [`BlockSink`]: a wait-free `rtrb` sample ring, a ring of [`Stamp`]s and
//! atomic counters, all allocated (and pre-faulted) before capture starts.
//! The reader drains the other end, [`BlockSource`], by polling; nothing on
//! the real-time side ever wakes it.
//!
//! The same ring carries audio at three points, told apart by what a stamp's
//! `at` means:
//!
//! - [`capture_ring`]: a device's native-rate blocks, stamped with the host
//!   time of their first frame (callback → conditioner);
//! - [`track_ring`] ([`RtSink`]): 48 kHz blocks, stamped with host time
//!   (conditioner → track writer);
//! - [`timeline_ring`] ([`TimelineSink`]): 48 kHz blocks as the writer placed
//!   them, stamped with their position on the recording timeline
//!   (track writer → live transcription).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rtrb::{Consumer, Producer, RingBuffer};

use crate::units::{Frames, HostTime, STORAGE_RATE};

/// Audio a ring holds before a slow reader makes the writer drop blocks.
pub const RING_DURATION: Duration = Duration::from_secs(2);

/// Placed audio the timeline ring holds for the live transcription reader.
pub const TIMELINE_RING_DURATION: Duration = Duration::from_secs(4);

/// Block stamps a ring holds: 2 s of 10 ms callbacks, with headroom for
/// devices that deliver smaller blocks.
pub const STAMP_CAPACITY: usize = 1024;

/// When (or where) one block starts, and how long it is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stamp<A> {
    /// The block's first frame: a host time or a timeline position.
    pub at: A,
    /// Frames in the block, at the ring's rate.
    pub len: u32,
}

/// A host-time stamp, as capture callbacks write them.
pub type BlockStamp = Stamp<HostTime>;

/// Counters the writing side bumps and the reader reads.
#[derive(Debug, Default)]
pub struct RtCounters {
    pushed: AtomicU64,
    dropped: AtomicU64,
}

impl RtCounters {
    /// Frames accepted into the ring so far.
    pub fn pushed(&self) -> Frames {
        Frames(self.pushed.load(Ordering::Relaxed))
    }

    /// Frames dropped because the ring was full.
    pub fn dropped(&self) -> Frames {
        Frames(self.dropped.load(Ordering::Relaxed))
    }
}

/// A sample format a capture callback may deliver.
pub trait CaptureSample: Copy {
    /// The sample as `f32` in `[-1, 1]`.
    fn to_f32(self) -> f32;
}

impl CaptureSample for f32 {
    fn to_f32(self) -> f32 {
        self
    }
}

impl CaptureSample for f64 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "audio samples in [-1, 1] lose only precision f32 cannot hold anyway"
    )]
    fn to_f32(self) -> f32 {
        self as f32
    }
}

impl CaptureSample for i16 {
    fn to_f32(self) -> f32 {
        f32::from(self) / 32_768.0
    }
}

impl CaptureSample for i32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "24 of a 32-bit sample's bits fit f32 exactly; the rest is below audibility"
    )]
    fn to_f32(self) -> f32 {
        self as f32 / 2_147_483_648.0
    }
}

/// The writing end of a ring. The only type a capture callback uses.
#[derive(Debug)]
pub struct BlockSink<A> {
    samples: Producer<f32>,
    stamps: Producer<Stamp<A>>,
    counters: Arc<RtCounters>,
}

/// The real-time end of a track's 48 kHz ring.
pub type RtSink = BlockSink<HostTime>;

/// The writer's end of the placed-audio ring.
pub type TimelineSink = BlockSink<Frames>;

impl<A: Copy> BlockSink<A> {
    /// Pushes one block of mono samples starting at `at`.
    ///
    /// Real-time safe: no allocation, no lock, no log, no system call. When
    /// the ring cannot take the whole block it drops it and counts the drop;
    /// the timeline turns the hole into silence. Returns whether it was kept.
    pub fn push(&mut self, samples: &[f32], at: A) -> bool {
        self.push_frames(samples, 1, at)
    }

    /// Pushes one block of interleaved `channels`-channel frames, averaging
    /// the channels into mono on the way in (Handy's `write_input_to_ring`).
    ///
    /// Real-time safe, like [`Self::push`]; a zero channel count drops nothing
    /// and pushes nothing.
    pub fn push_frames<S: CaptureSample>(&mut self, data: &[S], channels: usize, at: A) -> bool {
        let Some(frames) = data.len().checked_div(channels).filter(|f| *f > 0) else {
            return true;
        };
        let len = u32::try_from(frames).unwrap_or(u32::MAX);
        if self.stamps.is_full() || self.samples.slots() < frames {
            self.counters
                .dropped
                .fetch_add(u64::from(len), Ordering::Relaxed);
            return false;
        }
        let Ok(chunk) = self.samples.write_chunk_uninit(frames) else {
            return false;
        };
        chunk.fill_from_iter(data.chunks_exact(channels).map(mix_down));
        // The stamp goes second, so the reader never sees a stamp before its samples.
        let pushed = self.stamps.push(Stamp { at, len }).is_ok();
        self.counters
            .pushed
            .fetch_add(u64::from(len), Ordering::Relaxed);
        pushed
    }

    /// The counters shared with the reader.
    pub fn counters(&self) -> &Arc<RtCounters> {
        &self.counters
    }
}

/// The mean of one interleaved frame.
#[allow(
    clippy::cast_precision_loss,
    reason = "a frame has a handful of channels, far below f32's exact integer range"
)]
fn mix_down<S: CaptureSample>(frame: &[S]) -> f32 {
    let sum: f32 = frame.iter().map(|sample| sample.to_f32()).sum();
    sum / frame.len().max(1) as f32
}

/// The reading end of a ring.
#[derive(Debug)]
pub struct BlockSource<A> {
    samples: Consumer<f32>,
    stamps: Consumer<Stamp<A>>,
    counters: Arc<RtCounters>,
}

/// The writer's end of a track's 48 kHz ring.
pub type RtSource = BlockSource<HostTime>;

/// The live transcription's end of the placed-audio ring.
pub type TimelineSource = BlockSource<Frames>;

impl<A: Copy> BlockSource<A> {
    /// Pops the next whole block into `out` (cleared first) and returns its
    /// stamp, or `None` when no complete block is waiting.
    pub fn pop_block(&mut self, out: &mut Vec<f32>) -> Option<Stamp<A>> {
        let stamp = *self.stamps.peek().ok()?;
        let len = usize::try_from(stamp.len).unwrap_or(usize::MAX);
        out.clear();
        out.resize(len, 0.0);
        self.samples.pop_entire_slice(out).ok()?;
        self.stamps.pop().ok()
    }

    /// The counters shared with the writing side.
    pub fn counters(&self) -> &Arc<RtCounters> {
        &self.counters
    }

    /// Whether the writing side has been dropped (the source stopped).
    pub fn is_abandoned(&self) -> bool {
        self.stamps.is_abandoned()
    }
}

/// Builds a ring holding `capacity` samples, pre-faulted so the first
/// callbacks do not page-fault (Handy's `build_stream` pattern).
pub fn block_ring<A: Copy + Default>(capacity: usize) -> (BlockSink<A>, BlockSource<A>) {
    let (mut samples, mut sample_reader) = RingBuffer::<f32>::new(capacity);
    prefault(&mut samples, &mut sample_reader, capacity);
    let (stamps, stamp_reader) = RingBuffer::new(STAMP_CAPACITY);
    let counters = Arc::new(RtCounters::default());
    let sink = BlockSink {
        samples,
        stamps,
        counters: Arc::clone(&counters),
    };
    let source = BlockSource {
        samples: sample_reader,
        stamps: stamp_reader,
        counters,
    };
    (sink, source)
}

/// A track's 48 kHz ring, sized for [`RING_DURATION`].
pub fn track_ring() -> (RtSink, RtSource) {
    block_ring(Frames::from_duration(RING_DURATION).as_len())
}

/// A capture ring for a device delivering `rate` frames per second, sized
/// for [`RING_DURATION`] at that rate.
pub fn capture_ring(rate: u32) -> (RtSink, RtSource) {
    let rate = usize::try_from(rate.max(STORAGE_RATE)).unwrap_or(usize::MAX);
    let seconds = usize::try_from(RING_DURATION.as_secs()).unwrap_or(1);
    block_ring(rate.saturating_mul(seconds))
}

/// The placed-audio ring between a track writer and live transcription,
/// sized for [`TIMELINE_RING_DURATION`].
pub fn timeline_ring() -> (TimelineSink, TimelineSource) {
    block_ring(Frames::from_duration(TIMELINE_RING_DURATION).as_len())
}

/// Touches every slot once so the ring's pages are resident before capture.
fn prefault(writer: &mut Producer<f32>, reader: &mut Consumer<f32>, capacity: usize) {
    if let Ok(chunk) = writer.write_chunk_uninit(capacity) {
        chunk.fill_from_iter(std::iter::repeat(0.0));
    }
    if let Ok(chunk) = reader.read_chunk(capacity) {
        chunk.commit_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pushed_block_comes_out_whole_with_its_stamp() {
        let (mut sink, mut source) = track_ring();
        let mut out = Vec::new();

        assert!(sink.push(&[0.5; 480], HostTime(42)));
        let stamp = source.pop_block(&mut out);

        assert_eq!(
            stamp,
            Some(BlockStamp {
                at: HostTime(42),
                len: 480
            })
        );
        assert_eq!(out, vec![0.5; 480]);
        assert_eq!(source.pop_block(&mut out), None);
    }

    #[test]
    fn a_full_ring_drops_and_counts_the_block_instead_of_blocking() {
        let (mut sink, mut source) = track_ring();
        let second = vec![0.0; Frames::from_duration(Duration::from_secs(1)).as_len()];

        assert!(sink.push(&second, HostTime(0)));
        assert!(sink.push(&second, HostTime(1)));
        assert!(!sink.push(&[0.0; 480], HostTime(2)));

        assert_eq!(source.counters().dropped(), Frames(480));
        assert_eq!(source.counters().pushed(), Frames(96_000));
        let mut out = Vec::new();
        assert!(source.pop_block(&mut out).is_some());
    }

    #[test]
    fn interleaved_frames_are_averaged_into_mono() {
        let (mut sink, mut source) = capture_ring(44_100);
        let stereo: [i16; 4] = [16_384, 0, -16_384, -16_384];

        assert!(sink.push_frames(&stereo, 2, HostTime(7)));

        let mut out = Vec::new();
        let stamp = source.pop_block(&mut out).unwrap();
        assert_eq!(stamp.len, 2);
        assert!((out[0] - 0.25).abs() < 1e-6, "{}", out[0]);
        assert!((out[1] + 0.5).abs() < 1e-6, "{}", out[1]);
    }

    #[test]
    fn an_empty_block_or_no_channels_pushes_nothing() {
        let (mut sink, mut source) = capture_ring(48_000);
        assert!(sink.push_frames::<f32>(&[], 2, HostTime(0)));
        assert!(sink.push_frames(&[0.5_f32; 4], 0, HostTime(0)));
        assert_eq!(source.pop_block(&mut Vec::new()), None);
    }

    #[test]
    fn timeline_rings_carry_positions() {
        let (mut sink, mut source) = timeline_ring();
        assert!(sink.push(&[0.1; 160], Frames(4_800)));
        let stamp = source.pop_block(&mut Vec::new()).unwrap();
        assert_eq!(stamp.at, Frames(4_800));
    }

    #[test]
    fn the_writer_sees_when_the_source_goes_away() {
        let (sink, source) = track_ring();
        assert!(!source.is_abandoned());
        drop(sink);
        assert!(source.is_abandoned());
    }
}
