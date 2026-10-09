//! [`Detector`]: the thread that watches the Mac and feeds the service.
//!
//! OS listeners only wake it; it then re-reads every sensor, re-attaches
//! per-object listeners and evaluates (OpenWhispr's listener-and-reconcile
//! model). It also wakes at the service's next deadline (a dwell, a
//! countdown, a timeout) and on a heartbeat. When a listener cannot be
//! registered it falls back to polling every second, as anarlog does.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_domain::Clock;

use crate::calendar::CalendarLookup;
use crate::error::DetectError;
use crate::ports::{Sensors, Waker, Watch};
use crate::reader::SignalReader;
use crate::service::DetectionService;

/// With every listener registered: re-read anyway this often, in case a
/// notification was missed (OpenWhispr's heartbeat reconcile).
pub const HEARTBEAT: Duration = Duration::from_secs(5);

/// Without listeners: poll this often (anarlog's microphone poll).
pub const FALLBACK_POLL: Duration = Duration::from_secs(1);

/// The shortest wait, so a deadline a hair in the future does not spin.
const MIN_WAIT: Duration = Duration::from_millis(10);

/// What the detector thread works with, besides its sensors.
pub struct DetectorParts {
    /// The rules it feeds.
    pub service: Arc<DetectionService>,
    /// Calendar association.
    pub calendar: Arc<dyn CalendarLookup>,
    /// Time.
    pub clock: Arc<dyn Clock>,
}

/// The running detector. Dropping it stops the thread and its listeners.
pub struct Detector {
    waker: Waker,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Detector {
    /// Starts the detector thread. `sensors` builds the OS ports on that
    /// thread, where their listeners live and die.
    pub fn spawn<F>(sensors: F, parts: DetectorParts) -> Result<Self, DetectError>
    where
        F: FnOnce() -> Result<Sensors, DetectError> + Send + 'static,
    {
        let (waker, signals) = Waker::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let control = Control {
            waker: waker.clone(),
            signals,
            stop: Arc::clone(&stop),
        };
        let thread = std::thread::Builder::new()
            .name("deino-detect".to_owned())
            .spawn(move || run(sensors, parts, &control))
            .map_err(DetectError::Thread)?;
        Ok(Self {
            waker,
            stop,
            thread: Some(thread),
        })
    }

    /// Evaluates again now, for example after the settings changed.
    pub fn wake(&self) {
        self.waker.wake();
    }
}

impl Drop for Detector {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.waker.wake();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::error!(target: "detect", "the detector thread panicked");
        }
    }
}

/// How the thread is woken and stopped.
struct Control {
    waker: Waker,
    signals: Receiver<()>,
    stop: Arc<AtomicBool>,
}

fn run<F>(sensors: F, parts: DetectorParts, control: &Control)
where
    F: FnOnce() -> Result<Sensors, DetectError>,
{
    match sensors() {
        Ok(sensors) => Session::start(sensors, parts, &control.waker).run(control),
        Err(error) => log::warn!(target: "detect", "meeting detection is off: {error}"),
    }
}

/// The detector thread's state. Watches drop first, before the sensors.
struct Session {
    watches: Vec<Box<dyn Watch>>,
    sensors: Sensors,
    reader: SignalReader,
    parts: DetectorParts,
    poll: Duration,
}

impl Session {
    fn start(sensors: Sensors, parts: DetectorParts, waker: &Waker) -> Self {
        let results = [
            sensors.processes.watch(waker),
            sensors.microphone.watch(waker),
            sensors.camera.watch(waker),
        ];
        let mut watches = Vec::new();
        for result in results {
            match result {
                Ok(watch) => watches.push(watch),
                Err(error) => {
                    log::warn!(target: "detect", "listener unavailable, polling instead: {error}");
                }
            }
        }
        let poll = if watches.len() == 3 {
            HEARTBEAT
        } else {
            FALLBACK_POLL
        };
        log::info!(target: "detect", "meeting detection started ({} listeners)", watches.len());
        let reader = SignalReader::new(Arc::clone(&parts.calendar), Arc::clone(&parts.clock));
        Self {
            watches,
            sensors,
            reader,
            parts,
            poll,
        }
    }

    fn run(mut self, control: &Control) {
        while !control.stop.load(Ordering::Acquire) {
            self.tick();
            match control.signals.recv_timeout(self.wait()) {
                Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        log::info!(target: "detect", "meeting detection stopped");
    }

    fn tick(&mut self) {
        for watch in &mut self.watches {
            watch.refresh();
        }
        let snapshot = self.reader.read(&self.sensors);
        self.parts.service.evaluate(&snapshot);
    }

    /// Until the next deadline, at most one poll interval.
    fn wait(&self) -> Duration {
        let Some(deadline) = self.parts.service.next_deadline() else {
            return self.poll;
        };
        let until = (deadline - self.parts.clock.now())
            .to_std()
            .unwrap_or(Duration::ZERO);
        until.clamp(MIN_WAIT, self.poll)
    }
}

#[cfg(test)]
#[path = "detector_tests.rs"]
mod tests;
