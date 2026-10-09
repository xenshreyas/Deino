//! The headless benchmark behind `deino --transcribe-file file.wav`.
//!
//! Loads a model in a real worker process and measures, `repeat` times:
//!
//! - a **batch** run over the file in 25 s chunks (its real-time factor);
//! - a **live** run through the exact live path (`LiveSession`), feeding
//!   80 ms frames. Paced at real time (`realtime`), it reports the honest
//!   first-text and token latencies; unpaced, only the RTF is meaningful.
//!
//! The report is JSON-serializable and holds numbers only, unless the caller
//! asks for the live transcript (`include_text`) to check it by eye.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;

use crate::clock::MonotonicClock;
use crate::error::SttError;
use crate::pcm::{Millis, SAMPLE_RATE_HZ};
use crate::perf::PerfReport;
use crate::protocol::StreamParams;
use crate::session::{FinalSegment, LiveSession, LiveText, SessionConfig, SessionSink};
use crate::supervisor::{EngineSupervisor, LoadSpec, SupervisorConfig, WorkerLauncher};
use crate::window::StreamWindow;

/// Live frames are this long, like a capture callback's buffers.
const FRAME: Millis = Millis(80);

/// Batch runs take chunks of at most this length: Parakeet's one-shot run
/// can stop emitting text partway through long audio (STT spike, §4), so
/// import and retranscription feed it VAD segments of 25 s or less too.
const BATCH_CHUNK: Millis = Millis(25_000);

/// What to measure.
#[derive(Clone, Copy, Debug)]
pub struct BenchOptions {
    /// How many times to run each measurement.
    pub repeat: usize,
    /// Pace live audio at real time (needed for latency numbers).
    pub realtime: bool,
    /// The live stream window.
    pub window: Option<StreamWindow>,
    /// Put the live transcript in the report (for checking it by eye; the
    /// report is numbers only otherwise).
    pub include_text: bool,
}

/// One batch run.
#[derive(Clone, Debug, Serialize)]
pub struct BatchRun {
    /// Wall time of the run, in ms.
    pub wall_ms: u64,
    /// Wall time over audio time.
    pub rtf: f64,
    /// Words transcribed.
    pub words: usize,
}

/// One live run.
#[derive(Clone, Debug, Serialize)]
pub struct LiveRun {
    /// The lane's metrics.
    pub perf: PerfReport,
    /// Segments emitted.
    pub segments: usize,
    /// Words in those segments.
    pub words: usize,
    /// The segments, when [`BenchOptions::include_text`] asked for them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<Vec<TranscriptLine>>,
}

/// One live segment, for the opt-in transcript.
#[derive(Clone, Debug, Serialize)]
pub struct TranscriptLine {
    /// Start, in seconds from the start of the file.
    pub start_s: f64,
    /// End, in seconds.
    pub end_s: f64,
    /// The text.
    pub text: String,
}

/// Everything measured.
#[derive(Clone, Debug, Serialize)]
pub struct BenchReport {
    /// Audio length, in seconds.
    pub audio_s: f64,
    /// Spawn, backend init, load and warm-up, in ms.
    pub load_ms: u64,
    /// The backend the model bound to.
    pub backend: String,
    /// Whether the live runs were paced at real time.
    pub realtime: bool,
    /// Batch runs.
    pub batch: Vec<BatchRun>,
    /// Live runs.
    pub live: Vec<LiveRun>,
}

/// A WAV file the benchmark cannot read.
#[derive(Debug, thiserror::Error)]
pub enum WavError {
    /// The file is unreadable or not a WAV file.
    #[error("cannot read the WAV file: {0}")]
    Read(#[from] hound::Error),
    /// The audio is not 16 kHz mono.
    #[error("the WAV file must be 16 kHz mono (got {rate} Hz, {channels} channels)")]
    Format {
        /// Its sample rate.
        rate: u32,
        /// Its channel count.
        channels: u16,
    },
}

/// Reads a 16 kHz mono WAV file (16-bit integer or 32-bit float).
pub fn read_wav(path: &Path) -> Result<Vec<f32>, WavError> {
    let reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.sample_rate != SAMPLE_RATE_HZ || spec.channels != 1 {
        return Err(WavError::Format {
            rate: spec.sample_rate,
            channels: spec.channels,
        });
    }
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader.into_samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => reader
            .into_samples::<i16>()
            .map(|sample| sample.map(|value| f32::from(value) / 32_768.0))
            .collect::<Result<_, _>>()?,
    };
    Ok(samples)
}

/// The model to benchmark and how to start its worker.
#[derive(Clone, Debug)]
pub struct BenchTarget {
    /// Starts the worker process.
    pub launcher: WorkerLauncher,
    /// The model to load.
    pub spec: LoadSpec,
}

/// Loads the target's model in a fresh worker and measures `pcm`.
pub fn run(
    target: BenchTarget,
    pcm: &[f32],
    options: BenchOptions,
    clock: &Arc<dyn MonotonicClock>,
) -> Result<BenchReport, SttError> {
    let engine = EngineSupervisor::new(target.launcher, SupervisorConfig::default())?;
    let engine = Arc::new(engine);
    let started = clock.now();
    let info = engine.load(target.spec)?;
    let load_ms = millis(clock.now().saturating_sub(started));
    let audio = Millis::from_samples(pcm.len());
    let mut batch = Vec::new();
    let mut live = Vec::new();
    for _ in 0..options.repeat {
        batch.push(batch_run(&engine, pcm, audio, clock.as_ref())?);
        if info.supports_streaming {
            live.push(live_run(&engine, pcm, options, clock)?);
        }
    }
    Ok(BenchReport {
        audio_s: audio.as_secs_f64(),
        load_ms,
        backend: info.backend,
        realtime: options.realtime,
        batch,
        live,
    })
}

fn batch_run(
    engine: &EngineSupervisor,
    pcm: &[f32],
    audio: Millis,
    clock: &dyn MonotonicClock,
) -> Result<BatchRun, SttError> {
    let started = clock.now();
    let mut words = 0;
    for chunk in pcm.chunks(BATCH_CHUNK.to_samples()) {
        let transcript = engine.transcribe(chunk.to_vec())?;
        words += word_count(&transcript);
    }
    let wall = clock.now().saturating_sub(started);
    Ok(BatchRun {
        wall_ms: millis(wall),
        rtf: wall.as_secs_f64() / audio.as_secs_f64().max(f64::EPSILON),
        words,
    })
}

fn live_run(
    engine: &Arc<EngineSupervisor>,
    pcm: &[f32],
    options: BenchOptions,
    clock: &Arc<dyn MonotonicClock>,
) -> Result<LiveRun, SttError> {
    let segments = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Box::new(CollectingSink(Arc::clone(&segments)));
    let config = SessionConfig {
        label: "bench",
        params: StreamParams {
            window: options.window,
        },
        ..SessionConfig::default()
    };
    let session = LiveSession::start(engine.clone(), sink, Arc::clone(clock), config)?;
    let started = clock.now();
    for (index, frame) in pcm.chunks(FRAME.to_samples()).enumerate() {
        if options.realtime {
            pace(clock.as_ref(), started + frame_end(index));
        }
        session.feed(frame.to_vec());
    }
    let perf = session.finish();
    let segments = std::mem::take(
        &mut *segments
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    Ok(LiveRun {
        perf,
        segments: segments.len(),
        words: segments.iter().map(|segment| segment.words.len()).sum(),
        transcript: options.include_text.then(|| lines(&segments)),
    })
}

/// When frame `index` has been captured, measured from the first frame.
fn frame_end(index: usize) -> Duration {
    Duration::from_millis(FRAME.0).saturating_mul(u32::try_from(index + 1).unwrap_or(u32::MAX))
}

/// Waits until `due`, as a microphone would deliver the frame.
#[allow(
    clippy::disallowed_methods,
    reason = "the benchmark paces audio at real time on purpose"
)]
fn pace(clock: &dyn MonotonicClock, due: Duration) {
    if let Some(wait) = due.checked_sub(clock.now()) {
        std::thread::sleep(wait);
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Words in a transcript: from its tokens, or its text when the model
/// reports no token timestamps (Whisper `.bin` files).
fn word_count(transcript: &crate::protocol::BatchTranscript) -> usize {
    if transcript.tokens.is_empty() {
        transcript.text.split_whitespace().count()
    } else {
        crate::words::words_from_tokens(&transcript.tokens, Millis::ZERO).len()
    }
}

fn lines(segments: &[FinalSegment]) -> Vec<TranscriptLine> {
    segments
        .iter()
        .map(|segment| TranscriptLine {
            start_s: segment.start.as_secs_f64(),
            end_s: segment.end.as_secs_f64(),
            text: segment.text.clone(),
        })
        .collect()
}

/// Keeps the live lane's segments for counting (and the opt-in transcript).
struct CollectingSink(Arc<std::sync::Mutex<Vec<FinalSegment>>>);

impl SessionSink for CollectingSink {
    fn live(&mut self, _text: &LiveText) {}

    fn segment(&mut self, segment: FinalSegment) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(segment);
    }
}
