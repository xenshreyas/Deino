//! `MockAudioSource`: a real-time source that plays a [`TestSignal`].
//!
//! A thread produces one 10 ms block at a time, waits until the block's last
//! frame would have been captured, then pushes it stamped with the host time
//! of its first frame — the way a capture callback delivers audio. It waits
//! on its stop channel with a timeout, so stopping is immediate. A stopped
//! mock started again (a pause, a device restart) continues where it was.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use super::{AudioSource, SourceError, SourceFactory, SourceStream, TestSignal};
use crate::clock::HostClock;
use crate::ring::{RtSink, capture_ring};
use crate::units::{Frames, HostTime, SampleRate, Track};

/// Audio per delivered block, like a 10 ms device callback.
pub const MOCK_BLOCK: Duration = Duration::from_millis(10);

/// How late the mock system track starts, like a process tap that needs a
/// moment to attach. Exercises epoch anchoring end to end.
pub const MOCK_SYSTEM_DELAY: Duration = Duration::from_millis(300);

/// A running mock's thread and its stop channel.
#[derive(Debug)]
struct Running {
    stop: Sender<()>,
    thread: JoinHandle<()>,
}

/// Plays a generated signal into a track's ring in real time.
#[derive(Debug)]
pub struct MockAudioSource {
    track: Track,
    signal: TestSignal,
    clock: Arc<dyn HostClock>,
    delay: Duration,
    /// Where playback is, kept across restarts.
    position: Arc<AtomicU64>,
    running: Option<Running>,
}

impl MockAudioSource {
    /// A source for `track` playing `signal`, starting immediately.
    pub fn new(track: Track, signal: TestSignal, clock: Arc<dyn HostClock>) -> Self {
        Self {
            track,
            signal,
            clock,
            delay: Duration::ZERO,
            position: Arc::default(),
            running: None,
        }
    }

    /// Delays the first block by `delay` after [`AudioSource::start`].
    #[must_use]
    ///
    /// Like a tap attaching late to a live call, the signal is not shifted:
    /// what "played" during the delay is skipped, so the first block carries
    /// the signal at `delay`, and the track's content stays on the meeting's time.
    pub fn starting_after(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self.position = Arc::new(AtomicU64::new(Frames::from_duration(delay).0));
        self
    }

    /// A pretend meeting: a microphone voice, and a system voice that starts
    /// [`MOCK_SYSTEM_DELAY`] late.
    pub fn meeting(clock: &Arc<dyn HostClock>) -> [Self; 2] {
        Self::playing(clock, TestSignal::microphone(), TestSignal::system())
    }

    /// A pretend meeting playing the given voices, for example a fixture's
    /// tracks (`TestSignal::from_pcm` over `deino_testkit`'s `PcmTrack`).
    /// The system voice starts [`MOCK_SYSTEM_DELAY`] late.
    pub fn playing(
        clock: &Arc<dyn HostClock>,
        microphone: TestSignal,
        system: TestSignal,
    ) -> [Self; 2] {
        [
            Self::new(Track::Microphone, microphone, Arc::clone(clock)),
            Self::new(Track::System, system, Arc::clone(clock)).starting_after(MOCK_SYSTEM_DELAY),
        ]
    }
}

impl AudioSource for MockAudioSource {
    fn track(&self) -> Track {
        self.track
    }

    fn start(&mut self) -> Result<SourceStream, SourceError> {
        if self.running.is_some() {
            return Err(SourceError::AlreadyStarted(self.track));
        }
        let (sink, blocks) = capture_ring(SampleRate::STORAGE.0);
        let (stop, stops) = channel();
        let player = Player {
            signal: self.signal.clone(),
            clock: Arc::clone(&self.clock),
            first_block: self.clock.now().plus(self.delay),
            position: Arc::clone(&self.position),
            sink,
        };
        // Only the first start waits, like a tap attaching once.
        self.delay = Duration::ZERO;
        let thread = std::thread::Builder::new()
            .name(format!("deino-audio-mock-{}", self.track))
            .spawn(move || player.play(&stops))
            .map_err(|source| SourceError::Spawn {
                track: self.track,
                source,
            })?;
        self.running = Some(Running { stop, thread });
        Ok(SourceStream {
            rate: SampleRate::STORAGE,
            blocks,
        })
    }

    fn stop(&mut self) {
        let Some(running) = self.running.take() else {
            return;
        };
        // A send fails only if the thread already ended.
        running.stop.send(()).ok();
        if running.thread.join().is_err() {
            log::error!(target: "deino_audio::source", "the {} mock source panicked", self.track);
        }
    }
}

/// A [`SourceFactory`] of mock sources: a recording plays these voices
/// (generated, or a fixture's tracks), the system voice starting
/// [`MOCK_SYSTEM_DELAY`] late.
#[derive(Clone, Debug)]
pub struct MockSources {
    clock: Arc<dyn HostClock>,
    microphone: TestSignal,
    system: TestSignal,
}

impl MockSources {
    /// Mock sources playing `microphone` and `system`.
    pub fn new(clock: Arc<dyn HostClock>, microphone: TestSignal, system: TestSignal) -> Self {
        Self {
            clock,
            microphone,
            system,
        }
    }
}

impl SourceFactory for MockSources {
    fn microphone(&self) -> Box<dyn AudioSource> {
        let signal = self.microphone.clone();
        Box::new(MockAudioSource::new(
            Track::Microphone,
            signal,
            Arc::clone(&self.clock),
        ))
    }

    fn system(&self) -> Box<dyn AudioSource> {
        let signal = self.system.clone();
        let source = MockAudioSource::new(Track::System, signal, Arc::clone(&self.clock));
        Box::new(source.starting_after(MOCK_SYSTEM_DELAY))
    }
}

impl Drop for MockAudioSource {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The mock's thread state.
struct Player {
    signal: TestSignal,
    clock: Arc<dyn HostClock>,
    first_block: HostTime,
    position: Arc<AtomicU64>,
    sink: RtSink,
}

impl Player {
    /// Delivers blocks on schedule until told to stop.
    fn play(mut self, stops: &std::sync::mpsc::Receiver<()>) {
        let len = Frames::from_duration(MOCK_BLOCK);
        let mut block = vec![0.0; len.as_len()];
        let mut sent = Frames::ZERO;
        loop {
            let captured_at = self.first_block.plus(sent.as_duration());
            let delivered_at = captured_at.plus(MOCK_BLOCK);
            let wait = delivered_at.since(self.clock.now());
            if !matches!(stops.recv_timeout(wait), Err(RecvTimeoutError::Timeout)) {
                return;
            }
            let position = Frames(self.position.fetch_add(len.0, Ordering::AcqRel));
            self.signal.fill(position, &mut block);
            self.sink.push(&block, captured_at);
            sent += len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::SystemHostClock;

    #[test]
    fn the_mock_delivers_stamped_blocks_of_its_signal_and_stops_promptly() {
        let clock: Arc<dyn HostClock> = Arc::new(SystemHostClock::new());
        let signal = TestSignal::Tone {
            hz: 440.0,
            amplitude: 0.5,
        };
        let mut source = MockAudioSource::new(Track::Microphone, signal.clone(), clock);

        let SourceStream {
            rate,
            blocks: mut ring,
        } = source.start().unwrap();
        assert_eq!(rate, SampleRate::STORAGE);
        let mut out = Vec::new();
        let mut blocks = Vec::new();
        while blocks.len() < 3 {
            if let Some(stamp) = ring.pop_block(&mut out) {
                blocks.push((stamp, out.clone()));
            }
            std::thread::yield_now();
        }
        source.stop();

        let mut expected = vec![0.0; 480];
        signal.fill(Frames(480), &mut expected);
        assert_eq!(blocks[1].1, expected);
        let step = blocks[1].0.at.since(blocks[0].0.at);
        assert_eq!(step, MOCK_BLOCK);
        assert!(ring.is_abandoned(), "stopping releases the sink");
    }

    #[test]
    fn starting_twice_is_refused() {
        let clock: Arc<dyn HostClock> = Arc::new(SystemHostClock::new());
        let mut source = MockAudioSource::new(Track::System, TestSignal::system(), clock)
            .starting_after(Duration::from_hours(1));
        let _stream = source.start().unwrap();

        let error = source.start().unwrap_err();

        assert!(matches!(error, SourceError::AlreadyStarted(Track::System)));
    }

    #[test]
    fn a_restarted_mock_continues_where_it_stopped() {
        let clock: Arc<dyn HostClock> = Arc::new(SystemHostClock::new());
        let signal = TestSignal::Tone {
            hz: 440.0,
            amplitude: 0.5,
        };
        let mut source = MockAudioSource::new(Track::Microphone, signal.clone(), clock);
        let mut first = source.start().unwrap();
        let mut out = Vec::new();
        while first.blocks.pop_block(&mut out).is_none() {
            std::thread::yield_now();
        }
        source.stop();
        let delivered = source.position.load(Ordering::Acquire);

        let mut second = source.start().unwrap();
        while second.blocks.pop_block(&mut out).is_none() {
            std::thread::yield_now();
        }
        source.stop();

        let mut expected = vec![0.0; 480];
        signal.fill(Frames(delivered), &mut expected);
        assert_eq!(out, expected);
    }
}
