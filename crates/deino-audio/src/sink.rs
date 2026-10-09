//! Where encoded segments go: the `TrackSink` port and its file adapter.
//!
//! The encoder hands over each CMAF segment (init, then one `moof` + `mdat`
//! per second) as one byte slice. A [`TrackSink`] appends it in a single
//! write, so after `kill -9` the file ends on a segment boundary, and syncs
//! on demand, so the persister owns the fsync cadence. [`SegmentLedger`]
//! counts what the sink has accepted, so the manifest never claims audio
//! that is not in the file.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::durability::full_fsync;
use crate::units::Frames;

/// Receives a track's encoded segments.
///
/// Called from the encoder's own queue, so implementations are thread-safe.
pub trait TrackSink: std::fmt::Debug + Send + Sync {
    /// Appends one whole segment. The bytes must reach the OS before this
    /// returns (a `kill -9` afterwards keeps them).
    fn append(&self, segment: &[u8]) -> io::Result<()>;

    /// Makes everything appended so far survive a power loss.
    fn sync(&self) -> io::Result<()>;
}

/// Appends segments to `audio/<track>.NNN.m4a.part` on disk.
#[derive(Debug)]
pub struct PartFileSink {
    file: File,
    path: PathBuf,
}

impl PartFileSink {
    /// Creates (or truncates) the `.part` file at `path`.
    pub fn create(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        Ok(Self {
            file,
            path: path.to_owned(),
        })
    }

    /// The file being written.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl TrackSink for PartFileSink {
    fn append(&self, segment: &[u8]) -> io::Result<()> {
        // `&File` is `Write`: appends and the persister's fsync may overlap
        // safely, so no lock is needed.
        (&self.file).write_all(segment)
    }

    fn sync(&self) -> io::Result<()> {
        full_fsync(&self.file)
    }
}

/// What a track's sink has accepted so far. Shared by the encoder (which
/// records segments) and the persister (which reads it before syncing).
#[derive(Debug, Default)]
pub struct SegmentLedger {
    segments: AtomicU64,
    encoded_frames: AtomicU64,
    failed: AtomicBool,
}

impl SegmentLedger {
    /// Records one appended segment holding `frames` encoded frames.
    pub fn record(&self, frames: Frames) {
        self.segments.fetch_add(1, Ordering::AcqRel);
        self.encoded_frames.fetch_add(frames.0, Ordering::AcqRel);
    }

    /// Records that appending a segment failed.
    pub fn record_failure(&self) {
        self.failed.store(true, Ordering::Release);
    }

    /// Segments appended so far (the init segment included).
    pub fn segments(&self) -> u64 {
        self.segments.load(Ordering::Acquire)
    }

    /// Encoded frames (priming included) in the appended segments.
    pub fn encoded_frames(&self) -> Frames {
        Frames(self.encoded_frames.load(Ordering::Acquire))
    }

    /// Whether any append has failed.
    pub fn has_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use memory::MemorySink;

#[cfg(any(test, feature = "test-support"))]
mod memory {
    use std::io;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, PoisonError};

    use super::TrackSink;

    /// An in-memory sink: keeps every segment, counts syncs.
    #[derive(Debug, Default)]
    pub struct MemorySink {
        segments: Mutex<Vec<Vec<u8>>>,
        syncs: AtomicU64,
    }

    impl MemorySink {
        /// The segments appended so far, in order.
        pub fn segments(&self) -> Vec<Vec<u8>> {
            self.segments
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        /// How many times [`TrackSink::sync`] was called.
        pub fn syncs(&self) -> u64 {
            self.syncs.load(Ordering::SeqCst)
        }
    }

    impl TrackSink for MemorySink {
        fn append(&self, segment: &[u8]) -> io::Result<()> {
            let mut segments = self.segments.lock().unwrap_or_else(PoisonError::into_inner);
            segments.push(segment.to_vec());
            Ok(())
        }

        fn sync(&self) -> io::Result<()> {
            self.syncs.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_file_holds_segments_back_to_back() {
        let dir = tempfile::tempdir().unwrap();
        let sink = PartFileSink::create(&dir.path().join("mic.001.m4a.part")).unwrap();

        sink.append(b"init").unwrap();
        sink.append(b"media").unwrap();
        sink.sync().unwrap();

        assert_eq!(std::fs::read(sink.path()).unwrap(), b"initmedia");
    }

    #[test]
    fn the_ledger_adds_up_segments_and_frames() {
        let ledger = SegmentLedger::default();
        ledger.record(Frames::ZERO);
        ledger.record(Frames(48_128));

        assert_eq!(ledger.segments(), 2);
        assert_eq!(ledger.encoded_frames(), Frames(48_128));
        assert!(!ledger.has_failed());
        ledger.record_failure();
        assert!(ledger.has_failed());
    }

    #[test]
    fn the_memory_sink_keeps_segments_in_order() {
        let sink = MemorySink::default();
        sink.append(b"a").unwrap();
        sink.append(b"b").unwrap();
        sink.sync().unwrap();

        assert_eq!(sink.segments(), vec![b"a".to_vec(), b"b".to_vec()]);
        assert_eq!(sink.syncs(), 1);
    }
}
