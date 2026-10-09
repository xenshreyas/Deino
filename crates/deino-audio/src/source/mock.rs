//! `MockAudioSource`: a real-time source that plays a [`TestSignal`].
//!
//! A thread produces one 10 ms block at a time, waits until the block's last
//! frame would have been captured, then pushes it stamped with the host time
//! of its first frame — the way a capture callback delivers audio. It waits
//! on its stop channel with a timeout, so stopping is immediate.

use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use super::{AudioSource, SourceError, TestSignal};
use crate::clock::HostClock;
use crate::ring::RtSink;
use crate::units::{Frames, HostTime, Track};

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
            running: None,
        }
    }

    /// Delays the first block by `delay` after [`AudioSource::start`].
    #[must_use]
    pub fn starting_after(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// A pretend meeting: a microphone voice, and a system voice that starts
    /// [`MOCK_SYSTEM_DELAY`] late.
    pub fn meeting(clock: &Arc<dyn HostClock>) -> [Self; 2] {
        [
            Self::new(
                Track::Microphone,
                TestSignal::microphone(),
                Arc::clone(clock),
            ),
            Self::new(Track::System, TestSignal::system(), Arc::clone(clock))
                .starting_after(MOCK_SYSTEM_DELAY),
        ]
    }
}

impl AudioSource for MockAudioSource {
    fn track(&self) -> Track {
        self.track
    }

    fn start(&mut self, sink: RtSink) -> Result<(), SourceError> {
        if self.running.is_some() {
            return Err(SourceError::AlreadyStarted(self.track));
        }
        let (stop, stops) = channel();
        let player = Player {
            signal: self.signal.clone(),
            clock: Arc::clone(&self.clock),
            first_block: self.clock.now().plus(self.delay),
            sink,
        };
        let thread = std::thread::Builder::new()
            .name(format!("deino-audio-mock-{}", self.track))
            .spawn(move || player.play(&stops))
            .map_err(|source| SourceError::Spawn {
                track: self.track,
                source,
            })?;
        self.running = Some(Running { stop, thread });
        Ok(())
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
    sink: RtSink,
}

impl Player {
    /// Delivers blocks on schedule until told to stop.
    fn play(mut self, stops: &std::sync::mpsc::Receiver<()>) {
        let len = Frames::from_duration(MOCK_BLOCK);
        let mut block = vec![0.0; len.as_len()];
        let mut position = Frames::ZERO;
        loop {
            let captured_at = self.first_block.plus(position.as_duration());
            let delivered_at = captured_at.plus(MOCK_BLOCK);
            let wait = delivered_at.since(self.clock.now());
            if !matches!(stops.recv_timeout(wait), Err(RecvTimeoutError::Timeout)) {
                return;
            }
            self.signal.fill(position, &mut block);
            self.sink.push(&block, captured_at);
            position += len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::SystemHostClock;
    use crate::ring::track_ring;

    #[test]
    fn the_mock_delivers_stamped_blocks_of_its_signal_and_stops_promptly() {
        let clock: Arc<dyn HostClock> = Arc::new(SystemHostClock::new());
        let signal = TestSignal::Tone {
            hz: 440.0,
            amplitude: 0.5,
        };
        let mut source = MockAudioSource::new(Track::Microphone, signal.clone(), clock);
        let (sink, mut ring) = track_ring();

        source.start(sink).unwrap();
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
        source.start(track_ring().0).unwrap();

        let error = source.start(track_ring().0).unwrap_err();

        assert!(matches!(error, SourceError::AlreadyStarted(Track::System)));
    }
}
