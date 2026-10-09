//! The real-time boundary: a preallocated SPSC ring per track.
//!
//! A capture callback runs on a Core Audio real-time thread, where it must not
//! allocate, lock, log or make system calls (§3C). It only touches
//! [`RtSink`]: a wait-free `rtrb` sample ring, a ring of [`BlockStamp`]s and
//! atomic counters, all allocated (and pre-faulted) before capture starts.
//! The writer thread drains the other end, [`RtSource`], by polling; nothing
//! on the real-time side ever wakes it.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rtrb::{Consumer, Producer, RingBuffer};

use crate::units::{Frames, HostTime};

/// Audio the ring holds before a slow writer makes the source drop frames.
pub const RING_DURATION: Duration = Duration::from_secs(2);

/// Block stamps the ring holds: 2 s of 10 ms callbacks, with headroom for
/// devices that deliver smaller blocks.
pub const STAMP_CAPACITY: usize = 1024;

/// When and how much one capture callback delivered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlockStamp {
    /// Host time of the block's first frame.
    pub at: HostTime,
    /// Frames in the block (48 kHz mono).
    pub len: u32,
}

/// Counters the real-time side bumps and the writer reads.
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

/// The real-time end of a track's ring. The only type a capture callback uses.
#[derive(Debug)]
pub struct RtSink {
    samples: Producer<f32>,
    stamps: Producer<BlockStamp>,
    counters: Arc<RtCounters>,
}

impl RtSink {
    /// Pushes one block of 48 kHz mono samples captured at `at`.
    ///
    /// Real-time safe: no allocation, no lock, no log, no system call. When
    /// the ring cannot take the whole block it drops it and counts the drop;
    /// the timeline turns the hole into silence. Returns whether it was kept.
    pub fn push(&mut self, samples: &[f32], at: HostTime) -> bool {
        let len = u32::try_from(samples.len()).unwrap_or(u32::MAX);
        let frames = u64::from(len);
        if samples.is_empty() {
            return true;
        }
        if self.stamps.is_full() || self.samples.slots() < samples.len() {
            self.counters.dropped.fetch_add(frames, Ordering::Relaxed);
            return false;
        }
        // Both fit, so neither push can fail; the stamp goes second so the
        // reader never sees a stamp before its samples.
        let pushed = self.samples.push_entire_slice(samples).is_ok()
            && self.stamps.push(BlockStamp { at, len }).is_ok();
        self.counters.pushed.fetch_add(frames, Ordering::Relaxed);
        pushed
    }

    /// The counters shared with the writer.
    pub fn counters(&self) -> &Arc<RtCounters> {
        &self.counters
    }
}

/// The writer's end of a track's ring.
#[derive(Debug)]
pub struct RtSource {
    samples: Consumer<f32>,
    stamps: Consumer<BlockStamp>,
    counters: Arc<RtCounters>,
}

impl RtSource {
    /// Pops the next whole block into `out` (cleared first) and returns its
    /// stamp, or `None` when no complete block is waiting.
    pub fn pop_block(&mut self, out: &mut Vec<f32>) -> Option<BlockStamp> {
        let stamp = *self.stamps.peek().ok()?;
        let len = usize::try_from(stamp.len).unwrap_or(usize::MAX);
        out.clear();
        out.resize(len, 0.0);
        self.samples.pop_entire_slice(out).ok()?;
        self.stamps.pop().ok()
    }

    /// The counters shared with the real-time side.
    pub fn counters(&self) -> &Arc<RtCounters> {
        &self.counters
    }

    /// Whether the real-time side has been dropped (the source stopped).
    pub fn is_abandoned(&self) -> bool {
        self.stamps.is_abandoned()
    }
}

/// Builds a track's ring, sized for [`RING_DURATION`] and pre-faulted so the
/// first callbacks do not page-fault (Handy's `build_stream` pattern).
pub fn track_ring() -> (RtSink, RtSource) {
    let capacity = Frames::from_duration(RING_DURATION).as_len();
    let (mut samples, mut sample_reader) = RingBuffer::<f32>::new(capacity);
    prefault(&mut samples, &mut sample_reader, capacity);
    let (stamps, stamp_reader) = RingBuffer::new(STAMP_CAPACITY);
    let counters = Arc::new(RtCounters::default());
    let sink = RtSink {
        samples,
        stamps,
        counters: Arc::clone(&counters),
    };
    let source = RtSource {
        samples: sample_reader,
        stamps: stamp_reader,
        counters,
    };
    (sink, source)
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
    fn the_writer_sees_when_the_source_goes_away() {
        let (sink, source) = track_ring();
        assert!(!source.is_abandoned());
        drop(sink);
        assert!(source.is_abandoned());
    }
}
