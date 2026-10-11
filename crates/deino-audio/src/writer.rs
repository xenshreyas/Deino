//! The writer thread: one per track, the only code that touches the encoder.
//!
//! ```text
//!  conditioner ── RtSink ──▶ rtrb ring ──▶ writer thread (polls every 10 ms)
//!                                          TrackTimeline → Backlog → TrackEncoder
//!                                               │                    └─▶ TrackSink
//!                                               └─▶ TimelineSink (placed audio for
//!                                                   live transcription, never blocks)
//! ```
//!
//! The thread opens the encoder itself (native encoders are not `Send`),
//! polls the ring every [`POLL_INTERVAL`] (nothing on the real-time side ever
//! wakes it) and, on stop, pads the track to the stop time, drains the
//! backlog and finishes the encoder.

mod backlog;
mod pump;
mod stall;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender, channel, sync_channel};
use std::thread::JoinHandle;
use std::time::Duration;

pub use backlog::BACKLOG_LIMIT;
pub use pump::WriterReport;
pub use stall::STALL_THRESHOLD;

use self::pump::{PumpParts, TrackPump};
use crate::clock::HostClock;
use crate::encoder::{EncodeError, EncoderFactory, SegmentOutput};
use crate::health::HealthSender;
use crate::ring::{RtSource, TimelineSink};
use crate::units::{Frames, HostTime, Track};

/// How often the writer drains the ring (Handy's consumer cadence).
pub const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// How long a stopping writer waits for a refusing encoder before giving up
/// on the backlog.
pub const DRAIN_TIMEOUT: Duration = Duration::from_secs(10);

/// Why a writer failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WriterError {
    /// The encoder failed to open, encode or finish.
    #[error("the {track} encoder failed")]
    Encode {
        /// The failing track.
        track: Track,
        /// What went wrong.
        #[source]
        source: EncodeError,
    },
    /// The thread could not be started.
    #[error("could not start the {track} writer thread")]
    Spawn {
        /// The track.
        track: Track,
        /// The OS error.
        #[source]
        source: std::io::Error,
    },
    /// The thread panicked.
    #[error("the {0} writer thread stopped unexpectedly")]
    Panicked(Track),
}

/// One track's input to [`TrackWriter::spawn`].
#[derive(Debug)]
pub struct TrackWriterParts {
    /// The track to write.
    pub track: Track,
    /// The writer's end of the track's ring.
    pub source: RtSource,
    /// Where placed audio goes for live transcription.
    pub placed: TimelineSink,
    /// Where the encoder's segments go.
    pub output: SegmentOutput,
}

/// What every writer of a recording shares.
#[derive(Clone, Debug)]
pub struct WriterContext {
    /// Opens the encoders.
    pub encoders: Arc<dyn EncoderFactory>,
    /// Measures stalls.
    pub clock: Arc<dyn HostClock>,
    /// Receives stalls and drops.
    pub health: HealthSender,
    /// The recording epoch every timeline is anchored to.
    pub epoch: HostTime,
}

/// Stops a writer at a host time.
#[derive(Clone, Copy, Debug)]
struct Stop(HostTime);

/// A running writer thread.
#[derive(Debug)]
pub struct TrackWriter {
    track: Track,
    stop: Sender<Stop>,
    thread: JoinHandle<Result<WriterReport, EncodeError>>,
    appended: Arc<AtomicU64>,
}

impl TrackWriter {
    /// Starts the thread and waits until its encoder is open.
    pub fn spawn(parts: TrackWriterParts, context: &WriterContext) -> Result<Self, WriterError> {
        let track = parts.track;
        let appended = Arc::new(AtomicU64::new(0));
        let (stop, stops) = channel();
        let (opened, open_result) = sync_channel(1);
        let shared = (context.clone(), Arc::clone(&appended));
        let thread = std::thread::Builder::new()
            .name(format!("deino-audio-writer-{track}"))
            .spawn(move || run(parts, shared, &opened, &stops))
            .map_err(|source| WriterError::Spawn { track, source })?;
        let writer = Self {
            track,
            stop,
            thread,
            appended,
        };
        match open_result.recv() {
            Ok(()) => Ok(writer),
            Err(_) => Err(writer.join().err().unwrap_or(WriterError::Panicked(track))),
        }
    }

    /// The track this writer writes.
    pub fn track(&self) -> Track {
        self.track
    }

    /// Frames handed to the encoder so far.
    pub fn appended(&self) -> Frames {
        Frames(self.appended.load(Ordering::Acquire))
    }

    /// Stops at `at`: pads to it, drains, finishes the encoder, joins.
    pub fn stop(self, at: HostTime) -> Result<WriterReport, WriterError> {
        // A send fails only if the thread already ended; join reports why.
        self.stop.send(Stop(at)).ok();
        self.join()
    }

    /// Waits for the thread and maps its outcome.
    fn join(self) -> Result<WriterReport, WriterError> {
        let track = self.track;
        match self.thread.join() {
            Ok(Ok(report)) => Ok(report),
            Ok(Err(source)) => Err(WriterError::Encode { track, source }),
            Err(_) => Err(WriterError::Panicked(track)),
        }
    }
}

/// The thread body: open the encoder, poll until stopped, then drain.
fn run(
    parts: TrackWriterParts,
    (context, appended): (WriterContext, Arc<AtomicU64>),
    opened: &SyncSender<()>,
    stops: &Receiver<Stop>,
) -> Result<WriterReport, EncodeError> {
    // On failure `opened` is dropped unsent and `spawn` reads the error from
    // the joined thread.
    let encoder = context.encoders.open(parts.track, parts.output)?;
    opened.send(()).ok();
    let mut pump = TrackPump::new(PumpParts {
        track: parts.track,
        source: parts.source,
        placed: parts.placed,
        epoch: context.epoch,
        encoder,
        clock: context.clock,
        health: context.health,
        appended,
    });
    let stop_at = poll_until_stopped(&mut pump, stops)?;
    drain(pump, stop_at, stops)
}

/// Polls every [`POLL_INTERVAL`] until a stop arrives; returns its time.
fn poll_until_stopped(
    pump: &mut TrackPump,
    stops: &Receiver<Stop>,
) -> Result<HostTime, EncodeError> {
    loop {
        match stops.recv_timeout(POLL_INTERVAL) {
            Ok(Stop(at)) => return Ok(at),
            Err(RecvTimeoutError::Disconnected) => return Ok(pump.now()),
            Err(RecvTimeoutError::Timeout) => pump.poll()?,
        }
    }
}

/// Pads to `stop_at`, feeds the encoder until the backlog is empty (or the
/// encoder has refused audio for [`DRAIN_TIMEOUT`]) and finishes it.
fn drain(
    mut pump: TrackPump,
    stop_at: HostTime,
    stops: &Receiver<Stop>,
) -> Result<WriterReport, EncodeError> {
    pump.pad_to(stop_at);
    pump.feed()?;
    while !pump.is_drained() {
        let can_wait = matches!(
            stops.recv_timeout(POLL_INTERVAL),
            Err(RecvTimeoutError::Timeout)
        );
        if !can_wait || pump.waiting_for() > DRAIN_TIMEOUT {
            pump.abandon_backlog();
            break;
        }
        pump.settle()?;
    }
    pump.finish()
}
