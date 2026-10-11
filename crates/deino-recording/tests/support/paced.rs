//! A recording driven by sample count instead of wall time.
//!
//! [`PacedClock`] is the host clock of a fixture recording that runs as fast
//! as the pipeline can transcribe it, and never faster: host time is the
//! audio both lanes have been fed so far. The sources ([`PacedSources`])
//! deliver each 10 ms block once it is less than [`LOOKAHEAD`] ahead of host
//! time, so at most `LOOKAHEAD` of audio is ever between a source and the
//! lanes: less than any ring holds, so nothing is dropped, and a thread the
//! machine does not schedule for a while simply stops the clock instead of
//! letting its audio fall behind. Every timer the recorder measures on host
//! time (the capture watchdog, flushes, stalls) therefore sees audio time,
//! not the scheduler. The sources also stop at exactly the length the test
//! waits for ([`TestClock::wait_for`]), so the recording's end does not
//! depend on how soon the test gets to call stop: the transcript only
//! depends on the audio.
#![allow(dead_code, reason = "each test crate uses part of the shared support")]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_audio::capture::{
    FixedRoute, MicChoice, NoSystemWatch, OutputRoute, RouteProbe, SystemWatch,
};
use deino_audio::ring::{RtSink, capture_ring};
use deino_audio::source::MOCK_BLOCK;
use deino_audio::source::{AudioSource, SourceError, SourceFactory, SourceStream};
use deino_audio::{Frames, HostClock, HostTime, SampleRate, TestSignal, Track};
use deino_recording::engine::{EngineError, LaneTranscriber};
use deino_recording::testing::ScriptedEngines;
use deino_recording::vad::VoiceChange;
use deino_recording::{CaptureBackend, Lane, LaneEngines, LaneRequest};
use deino_stt::{Millis, StatusHandle};

use super::rig::{Harness, RigOptions, TestClock};

/// How far sources may run ahead of what the lanes have taken.
pub const LOOKAHEAD: Duration = Duration::from_millis(500);

/// How often a waiting source or test looks at the clock again.
const POLL: Duration = Duration::from_millis(1);

/// Host time paced by the lanes' intake.
#[derive(Debug)]
pub struct PacedClock {
    /// 16 kHz samples fed to each lane (Me, Them).
    fed: [AtomicU64; 2],
    /// Milliseconds each source has delivered (microphone, system).
    delivered: [AtomicU64; 2],
    /// Milliseconds each source plays before it goes quiet.
    limit: AtomicU64,
}

impl Default for PacedClock {
    fn default() -> Self {
        Self {
            fed: Default::default(),
            delivered: Default::default(),
            limit: AtomicU64::new(u64::MAX),
        }
    }
}

impl PacedClock {
    /// Audio both lanes have been fed.
    fn fed(&self) -> Duration {
        let samples = self.fed.iter().map(|fed| fed.load(Ordering::Acquire)).min();
        Duration::from_millis(samples.unwrap_or(0) / 16)
    }

    /// Milliseconds the slower source has delivered.
    fn delivered(&self) -> u64 {
        let delivered = self
            .delivered
            .iter()
            .map(|ms| ms.load(Ordering::Acquire))
            .min();
        delivered.unwrap_or(0)
    }

    fn take(&self, lane: Lane, samples: usize) {
        let index = usize::from(lane == Lane::Them);
        let samples = u64::try_from(samples).unwrap_or(u64::MAX);
        self.fed[index].fetch_add(samples, Ordering::AcqRel);
    }
}

/// Where host time starts.
const ORIGIN: HostTime = HostTime(1_000_000_000);

impl HostClock for PacedClock {
    fn now(&self) -> HostTime {
        ORIGIN.plus(self.fed())
    }
}

impl TestClock for PacedClock {
    /// Ends the sources at `audio` (rounded down to whole blocks) and
    /// waits until both have delivered it.
    fn wait_for(&self, audio: Duration) {
        let block = u64::try_from(MOCK_BLOCK.as_millis()).unwrap_or(1);
        let audio = u64::try_from(audio.as_millis()).unwrap_or(u64::MAX) / block * block;
        self.limit.store(audio, Ordering::Release);
        let (_keep, timer) = channel::<()>();
        while self.delivered() < audio {
            timer.recv_timeout(POLL).ok();
        }
    }
}

/// A fixture's two tracks played on the paced clock.
#[derive(Debug)]
struct PacedSources {
    clock: Arc<PacedClock>,
    microphone: TestSignal,
    system: TestSignal,
}

impl SourceFactory for PacedSources {
    fn microphone(&self) -> Box<dyn AudioSource> {
        Box::new(PacedSource::new(
            Track::Microphone,
            self.microphone.clone(),
            &self.clock,
        ))
    }

    fn system(&self) -> Box<dyn AudioSource> {
        Box::new(PacedSource::new(
            Track::System,
            self.system.clone(),
            &self.clock,
        ))
    }
}

/// One track's player: a block as soon as the clock passes it, never dropped.
#[derive(Debug)]
struct PacedSource {
    track: Track,
    signal: TestSignal,
    clock: Arc<PacedClock>,
    running: Option<(Sender<()>, JoinHandle<()>)>,
}

impl PacedSource {
    fn new(track: Track, signal: TestSignal, clock: &Arc<PacedClock>) -> Self {
        Self {
            track,
            signal,
            clock: Arc::clone(clock),
            running: None,
        }
    }
}

impl AudioSource for PacedSource {
    fn track(&self) -> Track {
        self.track
    }

    fn start(&mut self) -> Result<SourceStream, SourceError> {
        let (sink, blocks) = capture_ring(SampleRate::STORAGE.0);
        let (stop, stops) = channel();
        let player = Player {
            signal: self.signal.clone(),
            clock: Arc::clone(&self.clock),
            index: usize::from(self.track == Track::System),
        };
        let thread = std::thread::spawn(move || play(&player, sink, &stops));
        self.running = Some((stop, thread));
        Ok(SourceStream {
            rate: SampleRate::STORAGE,
            blocks,
        })
    }

    fn stop(&mut self) {
        if let Some((stop, thread)) = self.running.take() {
            stop.send(()).ok();
            thread.join().ok();
        }
    }
}

/// Pushes the signal block by block from the clock's time at start, each
/// once it is less than [`LOOKAHEAD`] ahead of the clock, until the limit.
fn play(source: &Player, mut sink: RtSink, stops: &std::sync::mpsc::Receiver<()>) {
    let clock = &source.clock;
    let len = Frames::from_duration(MOCK_BLOCK);
    let mut block = vec![0.0; len.as_len()];
    let (first, mut sent) = (clock.now(), Frames::ZERO);
    let waiting = |ready: &mut dyn FnMut() -> bool| {
        while !ready() {
            if !matches!(stops.recv_timeout(POLL), Err(RecvTimeoutError::Timeout)) {
                return false;
            }
        }
        true
    };
    loop {
        let captured_at = first.plus(sent.as_duration());
        let end = sent.as_duration() + MOCK_BLOCK;
        let mut due = || {
            let limit = Duration::from_millis(clock.limit.load(Ordering::Acquire));
            end <= limit && captured_at.plus(MOCK_BLOCK) <= clock.now().plus(LOOKAHEAD)
        };
        if !waiting(&mut due) {
            return;
        }
        source.signal.fill(sent, &mut block);
        if !waiting(&mut || sink.push(&block, captured_at)) {
            return;
        }
        sent += len;
        let delivered = u64::try_from(end.as_millis()).unwrap_or(u64::MAX);
        clock.delivered[source.index].store(delivered, Ordering::Release);
    }
}

/// What a source thread plays and where it reports.
struct Player {
    signal: TestSignal,
    clock: Arc<PacedClock>,
    /// 0 for the microphone, 1 for system audio.
    index: usize,
}

/// Capture over the paced sources.
#[derive(Debug)]
struct PacedCapture {
    sources: Arc<PacedSources>,
    route: OutputRoute,
}

impl CaptureBackend for PacedCapture {
    fn sources(&self, _microphone: &MicChoice) -> Arc<dyn SourceFactory> {
        Arc::clone(&self.sources) as Arc<dyn SourceFactory>
    }

    fn watch(&self) -> Arc<dyn SystemWatch> {
        Arc::new(NoSystemWatch)
    }

    fn route(&self) -> Arc<dyn RouteProbe> {
        Arc::new(FixedRoute(Some(self.route)))
    }
}

/// The scripted engines, telling the clock what each lane takes.
struct CountingEngines {
    clock: Arc<PacedClock>,
}

impl LaneEngines for CountingEngines {
    fn open(&self, request: LaneRequest) -> Result<Box<dyn LaneTranscriber>, EngineError> {
        let lane = request.lane;
        let inner = ScriptedEngines.open(request)?;
        Ok(Box::new(Counting {
            lane,
            inner,
            clock: Arc::clone(&self.clock),
        }))
    }
}

/// A lane transcriber whose intake moves the clock.
struct Counting {
    lane: Lane,
    inner: Box<dyn LaneTranscriber>,
    clock: Arc<PacedClock>,
}

impl LaneTranscriber for Counting {
    fn feed(&mut self, pcm: Vec<f32>) {
        let samples = pcm.len();
        self.inner.feed(pcm);
        self.clock.take(self.lane, samples);
    }

    fn voice(&mut self, change: VoiceChange) {
        self.inner.voice(change);
    }

    fn backlog(&self) -> Millis {
        self.inner.backlog()
    }

    fn status(&self) -> Option<StatusHandle> {
        self.inner.status()
    }

    fn finish(self: Box<Self>) {
        self.inner.finish();
    }
}

/// A fixture recording of `microphone` and `system` on `route`, paced by
/// the lanes' intake.
pub fn paced(
    microphone: TestSignal,
    system: TestSignal,
    route: OutputRoute,
) -> (Harness, RigOptions) {
    let clock = Arc::new(PacedClock::default());
    let sources = Arc::new(PacedSources {
        clock: Arc::clone(&clock),
        microphone,
        system,
    });
    let harness = Harness {
        clock: clock.clone(),
        host: clock.clone(),
        capture: Arc::new(PacedCapture { sources, route }),
    };
    let options = RigOptions {
        engines: Arc::new(CountingEngines { clock }),
        ..RigOptions::default()
    };
    (harness, options)
}
