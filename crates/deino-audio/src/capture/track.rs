//! `TrackCapture`: one track's source and its conditioner thread.
//!
//! The track writer's [`RtSink`] belongs to the conditioner thread for the
//! whole recording, so the timeline stays continuous while the device behind
//! it changes: stopping and restarting a source (a pause, a device change, a
//! recovery attempt) hands the thread a new capture ring, which it switches
//! to after draining the old one. The thread polls every [`POLL_INTERVAL`];
//! the real-time side never wakes it.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;

use super::conditioner::{CaptureShared, Conditioner};
use super::meter::TrackMeter;
use crate::ring::{RtSink, RtSource};
use crate::source::{AudioSource, SourceError, SourceStream};
use crate::units::Track;
use crate::writer::POLL_INTERVAL;

/// What the conditioner thread is told.
enum Control {
    /// Switch to a restarted source's ring (after draining the current one).
    Input(SourceStream),
    /// Drain and exit.
    Stop,
}

/// A track's capture: the source, and the thread conditioning its audio.
#[derive(Debug)]
pub struct TrackCapture {
    track: Track,
    source: Box<dyn AudioSource>,
    running: bool,
    control: Sender<Control>,
    thread: Option<JoinHandle<()>>,
    meter: Arc<TrackMeter>,
}

impl TrackCapture {
    /// Starts `source` and a conditioner feeding `output`, the track
    /// writer's ring.
    pub fn start(
        mut source: Box<dyn AudioSource>,
        output: RtSink,
        shared: &CaptureShared,
    ) -> Result<Self, SourceError> {
        let stream = source.start()?;
        Self::spawn(source, stream, output, shared)
    }

    /// Runs the conditioner for an already started `source` delivering
    /// `stream` (so a caller whose source fails to start keeps `output`).
    pub fn spawn(
        source: Box<dyn AudioSource>,
        stream: SourceStream,
        output: RtSink,
        shared: &CaptureShared,
    ) -> Result<Self, SourceError> {
        let track = source.track();
        let meter = Arc::new(TrackMeter::default());
        let conditioner = Conditioner::new(stream.rate, shared, Arc::clone(&meter))
            .map_err(|error| open_error(track, error))?;
        let (control, controls) = channel();
        let worker = Worker {
            conditioner,
            shared: shared.clone(),
            input: stream.blocks,
            output,
        };
        let thread = std::thread::Builder::new()
            .name(format!("deino-audio-condition-{track}"))
            .spawn(move || worker.run(&controls))
            .map_err(|source| SourceError::Spawn { track, source })?;
        Ok(Self {
            track,
            source,
            running: true,
            control,
            thread: Some(thread),
            meter,
        })
    }

    /// The track.
    pub fn track(&self) -> Track {
        self.track
    }

    /// The conditioner's activity counters.
    pub fn meter(&self) -> &Arc<TrackMeter> {
        &self.meter
    }

    /// Whether the source is delivering (not suspended, and not failed).
    pub fn is_alive(&self) -> bool {
        self.running && self.source.is_alive()
    }

    /// Whether the source is meant to be running (not suspended).
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Whether the open device is a Bluetooth headset in call mode.
    pub fn hands_free(&self) -> bool {
        self.running && self.source.hands_free()
    }

    /// Stops the source, keeping the conditioner (a pause, sleep).
    pub fn suspend(&mut self) {
        self.source.stop();
        self.running = false;
    }

    /// Starts the current source again (resume, a recovery attempt).
    pub fn restart(&mut self) -> Result<(), SourceError> {
        self.source.stop();
        self.running = false;
        let stream = self.source.start()?;
        self.running = true;
        self.switch_to(stream);
        Ok(())
    }

    /// Replaces the source with `source` and starts it (a device change).
    pub fn replace(&mut self, source: Box<dyn AudioSource>) -> Result<(), SourceError> {
        self.source.stop();
        self.source = source;
        self.restart()
    }

    /// Stops the source, drains the conditioner and joins its thread.
    pub fn finish(mut self) {
        self.shut_down();
    }

    fn switch_to(&self, stream: SourceStream) {
        // A send fails only if the thread already ended; `finish` reports it.
        self.control.send(Control::Input(stream)).ok();
    }

    fn shut_down(&mut self) {
        self.source.stop();
        self.running = false;
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.control.send(Control::Stop).ok();
        if thread.join().is_err() {
            log::error!(target: "deino_audio::capture", "the {} conditioner panicked", self.track);
        }
    }
}

impl Drop for TrackCapture {
    fn drop(&mut self) {
        self.shut_down();
    }
}

/// Wraps a resampler failure as a source error.
fn open_error(track: Track, error: impl std::error::Error + Send + Sync + 'static) -> SourceError {
    SourceError::Open {
        track,
        source: Box::new(error),
    }
}

/// The conditioner thread's state.
struct Worker {
    conditioner: Conditioner,
    shared: CaptureShared,
    input: RtSource,
    output: RtSink,
}

impl Worker {
    /// Pumps every [`POLL_INTERVAL`] until stopped; drains before exiting.
    fn run(mut self, controls: &Receiver<Control>) {
        loop {
            match controls.recv_timeout(POLL_INTERVAL) {
                Err(RecvTimeoutError::Timeout) => {
                    self.conditioner.pump(&mut self.input, &mut self.output);
                }
                Ok(Control::Input(stream)) => self.switch(stream),
                Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected) => {
                    self.conditioner.pump(&mut self.input, &mut self.output);
                    return;
                }
            }
        }
    }

    /// Drains the old ring, then conditions the new one at its rate.
    fn switch(&mut self, stream: SourceStream) {
        self.conditioner.pump(&mut self.input, &mut self.output);
        if stream.rate != self.conditioner.rate() {
            let meter = Arc::clone(self.conditioner.meter());
            match Conditioner::new(stream.rate, &self.shared, meter) {
                Ok(conditioner) => self.conditioner = conditioner,
                Err(error) => log::error!(target: "deino_audio::capture", "{error}"),
            }
        }
        self.input = stream.blocks;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::clock::{HostClock, SystemHostClock};
    use crate::ring::track_ring;
    use crate::source::{MockAudioSource, TestSignal};

    fn wait_for_blocks(track: &mut RtSource, count: usize) -> usize {
        let (_keep, timer) = channel::<()>();
        let mut seen = 0;
        let mut out = Vec::new();
        for _ in 0..500 {
            while track.pop_block(&mut out).is_some() {
                seen += 1;
            }
            if seen >= count {
                break;
            }
            timer.recv_timeout(Duration::from_millis(5)).ok();
        }
        seen
    }

    #[test]
    fn a_track_keeps_its_writer_ring_across_restarts() {
        let clock: Arc<dyn HostClock> = Arc::new(SystemHostClock::new());
        let source = MockAudioSource::new(Track::Microphone, TestSignal::microphone(), clock);
        let (output, mut track) = track_ring();
        let mut capture =
            TrackCapture::start(Box::new(source), output, &CaptureShared::default()).unwrap();
        assert!(wait_for_blocks(&mut track, 3) >= 3);

        capture.suspend();
        assert!(!capture.is_alive());
        capture.restart().unwrap();

        assert!(capture.is_alive());
        assert!(wait_for_blocks(&mut track, 3) >= 3);
        assert!(capture.meter().frames().0 >= 2_880);
        capture.finish();
        assert!(track.is_abandoned(), "finishing releases the writer's ring");
    }
}
