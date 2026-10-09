//! The `TrackEncoder` port: PCM in, encoded segments out to a `TrackSink`.
//!
//! The writer thread owns one encoder per track and feeds it 48 kHz mono f32
//! with the timeline position of each block (the sample index is time). The
//! encoder hands every finished segment to its [`SegmentOutput`], which
//! appends it to the sink and records it in the ledger.
//!
//! - [`CmafEncoderFactory`] (macOS): `AVAssetWriter` in CMAF segment-output
//!   mode, AAC-LC 64 kbps, 1 s segments (ADR 0014).
//! - `FakeEncoderFactory` (`test-support`): keeps the PCM in memory.

#[cfg(target_os = "macos")]
mod cmaf;
#[cfg(any(test, feature = "test-support"))]
mod fake;

use std::sync::Arc;

#[cfg(target_os = "macos")]
pub use cmaf::{BIT_RATE, CmafEncoderFactory, SEGMENT_INTERVAL};
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeEncoderFactory;

use crate::fmp4::{AAC_FRAMES_PER_PACKET, media_packets};
use crate::sink::{SegmentLedger, TrackSink};
use crate::units::{Frames, Track};

/// Why an encoder failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EncodeError {
    /// A Core Audio or Core Media call returned an `OSStatus`.
    #[error("{call} failed with OSStatus {status}")]
    Os {
        /// The failing call.
        call: &'static str,
        /// The status it returned.
        status: i32,
    },
    /// `AVAssetWriter` reported an error (its domain code; never content).
    #[error("{call} failed with AVFoundation error {code}")]
    Writer {
        /// The failing call.
        call: &'static str,
        /// The `NSError` code.
        code: isize,
    },
    /// The encoder rejected input or a call without saying why.
    #[error("{0} was rejected by the encoder")]
    Rejected(&'static str),
    /// Finishing the file did not complete in time.
    #[error("the encoder did not finish in time")]
    Timeout,
    /// Native encoding is not available on this platform.
    #[error("native audio encoding is only available on macOS")]
    Unsupported,
}

/// Encodes one track. Created and used on the writer thread only.
pub trait TrackEncoder {
    /// Whether the encoder can take more audio right now. When it cannot, the
    /// writer keeps audio in its backlog and tries again on the next poll.
    fn is_ready(&self) -> bool;

    /// Encodes `samples` (48 kHz mono), whose first frame is timeline frame `at`.
    fn append(&mut self, samples: &[f32], at: Frames) -> Result<(), EncodeError>;

    /// Emits the last (partial) segment and closes the stream.
    fn finish(self: Box<Self>) -> Result<(), EncodeError>;
}

/// Opens a track encoder. Called on the writer thread, because native
/// encoders are not `Send`.
pub trait EncoderFactory: std::fmt::Debug + Send + Sync {
    /// An encoder for `track` that delivers its segments to `output`.
    fn open(
        &self,
        track: Track,
        output: SegmentOutput,
    ) -> Result<Box<dyn TrackEncoder>, EncodeError>;
}

/// Where an encoder's segments go: the sink, and the ledger that counts them.
#[derive(Clone, Debug)]
pub struct SegmentOutput {
    /// Receives the segment bytes.
    pub sink: Arc<dyn TrackSink>,
    /// Counts what the sink accepted.
    pub ledger: Arc<SegmentLedger>,
}

impl SegmentOutput {
    /// Appends one segment and records its frames, or records the failure.
    /// Runs on the encoder's queue: it never panics and logs only counts.
    pub fn deliver(&self, segment: &[u8]) {
        match self.sink.append(segment) {
            Ok(()) => {
                let packets = media_packets(segment);
                self.ledger.record(Frames(packets * AAC_FRAMES_PER_PACKET));
            }
            Err(error) => {
                self.ledger.record_failure();
                log::error!(
                    target: "deino_audio::encoder",
                    "could not append a {}-byte segment: {error}",
                    segment.len()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;
    use crate::fmp4_fixture::{init_segment, media_segment};
    use crate::sink::MemorySink;

    #[derive(Debug)]
    struct FullDisk;

    impl TrackSink for FullDisk {
        fn append(&self, _segment: &[u8]) -> io::Result<()> {
            Err(io::ErrorKind::StorageFull.into())
        }

        fn sync(&self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn delivered_segments_reach_the_sink_and_count_their_packets() {
        let sink = Arc::new(MemorySink::default());
        let output = SegmentOutput {
            sink: sink.clone(),
            ledger: Arc::default(),
        };

        output.deliver(&init_segment());
        output.deliver(&media_segment(47));

        assert_eq!(sink.segments().len(), 2);
        assert_eq!(output.ledger.segments(), 2);
        assert_eq!(output.ledger.encoded_frames(), Frames(47 * 1024));
    }

    #[test]
    fn a_failed_append_is_recorded_not_counted() {
        let output = SegmentOutput {
            sink: Arc::new(FullDisk),
            ledger: Arc::default(),
        };

        output.deliver(&media_segment(47));

        assert!(output.ledger.has_failed());
        assert_eq!(output.ledger.encoded_frames(), Frames::ZERO);
    }
}
