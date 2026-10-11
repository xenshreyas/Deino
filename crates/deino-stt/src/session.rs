//! A live lane: continuous audio in, live text and final segments out.
//!
//! [`LiveSession`] runs one lane (microphone, system, or both mixed) on its
//! own thread, fed through a FIFO channel, so feeds and the final flush keep
//! their order and the capture side never waits on the engine.
//!
//! - **Continuous audio.** Audio is never VAD-gated, so the stream's clock is
//!   the recording's clock and token times land on meeting time.
//! - **Live and final text.** The open segment's committed text (plus any
//!   tentative tail) goes to [`SessionSink::live`] as it grows; a segment
//!   that closes goes to [`SessionSink::segment`] with its timed words.
//! - **Batch fallback.** The audio since the last closed segment is
//!   retained. When the stream is lost (worker crash, hang, engine error) or
//!   finalize fails, that audio is transcribed in one run and a new stream
//!   opens where the old one stopped. Nothing said is lost.
//! - **Rotation.** After [`SessionConfig::rotate_after`] of audio the stream
//!   is finalized and reopened at the next silence (at
//!   [`SessionConfig::rotate_limit`] even mid-speech). transcribe-cpp's
//!   buffered stream never trims its PCM buffer (about 230 MB per hour) and
//!   rebuilds its text on every chunk, so rolling keeps both bounded; token
//!   times stay continuous because each stream records where it began.

mod assembler;
mod retained;
mod runner;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub use assembler::SegmentPolicy;
use serde::Serialize;

use crate::clock::MonotonicClock;
use crate::error::SttError;
use crate::pcm::Millis;
use crate::perf::PerfReport;
use crate::protocol::StreamParams;
use crate::transcriber::Transcriber;
use crate::window::StreamWindow;
use crate::words::TimedWord;

/// The open segment's text, as it grows.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct LiveText {
    /// When the open segment started; zero when there is none.
    pub start: Millis,
    /// Committed text: final, never rewritten.
    pub committed: String,
    /// The uncommitted tail, which may still change.
    pub tentative: String,
}

/// How a final segment was transcribed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SegmentOrigin {
    /// By the live stream.
    Stream,
    /// By a batch run over retained audio after the stream was lost.
    BatchFallback,
}

/// A closed segment on the recording's timeline.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FinalSegment {
    /// When its first word starts.
    pub start: Millis,
    /// When its last word ends.
    pub end: Millis,
    /// Its text.
    pub text: String,
    /// Its words with their times.
    pub words: Vec<TimedWord>,
    /// How it was transcribed.
    pub origin: SegmentOrigin,
}

/// Where a session delivers text. Called on the session's thread.
pub trait SessionSink: Send {
    /// The open segment changed (an empty text means no segment is open).
    fn live(&mut self, text: &LiveText);
    /// A segment closed.
    fn segment(&mut self, segment: FinalSegment);
    /// The lane's transcript is complete up to `through`: every word that
    /// ends before it has been delivered (as live or final text), and no
    /// text for earlier audio will follow. Called after the text it
    /// covers, never with a smaller value than before. A consumer that waits
    /// for the other lane's words (the bleed holdback) waits on this
    /// watermark rather than on a clock, so a lane that falls behind delays
    /// the judgement instead of falsifying it.
    fn progress(&mut self, _through: Millis) {}
}

/// What a running session reports about its stream, for recording health.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LaneStatus {
    /// Times the stream was lost (worker crash, hang, engine error) and the
    /// lane fell back to a batch run.
    pub recoveries: u32,
    /// Whether a live stream is open. False after a loss until a new stream
    /// opens; the lane then transcribes in batches.
    pub streaming: bool,
}

/// Reads a running session's [`LaneStatus`] from any thread.
#[derive(Clone, Debug)]
pub struct StatusHandle(Arc<SharedStatus>);

impl StatusHandle {
    /// The session's status now.
    pub fn read(&self) -> LaneStatus {
        self.0.read()
    }
}

/// [`LaneStatus`] shared between the session thread and its owner.
#[derive(Debug, Default)]
pub(crate) struct SharedStatus {
    recoveries: AtomicU32,
    streaming: AtomicBool,
}

impl SharedStatus {
    fn read(&self) -> LaneStatus {
        LaneStatus {
            recoveries: self.recoveries.load(Ordering::Relaxed),
            streaming: self.streaming.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn set_streaming(&self, streaming: bool) {
        self.streaming.store(streaming, Ordering::Relaxed);
    }

    pub(crate) fn recovered(&self) {
        self.recoveries.fetch_add(1, Ordering::Relaxed);
    }
}

/// How a session runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionConfig {
    /// The lane's name in logs.
    pub label: &'static str,
    /// Stream options.
    pub params: StreamParams,
    /// When segments close.
    pub policy: SegmentPolicy,
    /// Where the session starts on the recording's timeline.
    pub start: Millis,
    /// Finalize and reopen the stream after this much audio, at a silence.
    pub rotate_after: Millis,
    /// Finalize and reopen it after this much audio, silence or not.
    pub rotate_limit: Millis,
    /// How often `StreamPerf` logs.
    pub perf_interval: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            label: "mixed",
            params: StreamParams {
                window: Some(StreamWindow::LIVE),
            },
            policy: SegmentPolicy::default(),
            start: Millis::ZERO,
            rotate_after: Millis(3 * 60 * 1000),
            rotate_limit: Millis(5 * 60 * 1000),
            perf_interval: Duration::from_secs(5),
        }
    }
}

/// Input for the session thread.
enum Input {
    Audio(Vec<f32>),
    Finish,
}

/// A running live lane.
pub struct LiveSession {
    input: Sender<Input>,
    thread: Option<JoinHandle<PerfReport>>,
    /// Samples queued for the session thread.
    queued: Arc<AtomicUsize>,
    /// The stream's health, written by the session thread.
    status: Arc<SharedStatus>,
}

impl LiveSession {
    /// Opens the stream (so a missing model fails here, not mid-meeting) and
    /// starts the session thread.
    pub fn start(
        transcriber: Arc<dyn Transcriber>,
        sink: Box<dyn SessionSink>,
        clock: Arc<dyn MonotonicClock>,
        config: SessionConfig,
    ) -> Result<Self, SttError> {
        let stream = transcriber.open_stream(config.params)?;
        let queued = Arc::new(AtomicUsize::new(0));
        let status = Arc::new(SharedStatus::default());
        status.set_streaming(true);
        let runner = runner::Runner::new(runner::Parts {
            transcriber,
            stream,
            sink,
            clock,
            config,
            queued: Arc::clone(&queued),
            status: Arc::clone(&status),
        });
        let (input, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(format!("stt-lane-{}", config.label))
            .spawn(move || runner.run(&receiver))
            .map_err(SttError::Spawn)?;
        Ok(Self {
            input,
            thread: Some(thread),
            queued,
            status,
        })
    }

    /// Queues 16 kHz mono audio, in capture order. Never blocks.
    pub fn feed(&self, pcm: Vec<f32>) {
        let samples = pcm.len();
        self.queued.fetch_add(samples, Ordering::Relaxed);
        if self.input.send(Input::Audio(pcm)).is_err() {
            self.queued.fetch_sub(samples, Ordering::Relaxed);
            log::warn!("live transcription lane stopped; dropping fed audio");
        }
    }

    /// Audio queued and not yet fed to the engine.
    pub fn backlog(&self) -> Millis {
        Millis::from_samples(self.queued.load(Ordering::Relaxed))
    }

    /// Whether the stream is open and how often it was lost.
    pub fn status(&self) -> LaneStatus {
        self.status.read()
    }

    /// A handle that reads [`Self::status`] from another thread for as
    /// long as the session lives (recording health polls it).
    pub fn status_handle(&self) -> StatusHandle {
        StatusHandle(Arc::clone(&self.status))
    }

    /// Transcribes everything fed so far, flushes the last segment and stops.
    /// Blocks until done; returns the lane's metrics.
    pub fn finish(mut self) -> PerfReport {
        self.stop().unwrap_or_default()
    }

    fn stop(&mut self) -> Option<PerfReport> {
        let thread = self.thread.take()?;
        if self.input.send(Input::Finish).is_err() {
            log::debug!("live transcription lane already stopped");
        }
        thread.join().ok()
    }
}

impl Drop for LiveSession {
    fn drop(&mut self) {
        self.stop();
    }
}
