//! [`AutoStopTicker`]: the thread that ticks [`AutoStopService`] once a
//! [`TICK`] and passes on sleep notifications. Dropping it stops the thread.

use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use super::service::AutoStopService;
use crate::error::DetectError;

/// How often the service looks at the recording.
pub const TICK: Duration = Duration::from_secs(1);

/// What the thread is told besides its ticks.
enum Signal {
    WillSleep,
}

/// The running ticker.
pub struct AutoStopTicker {
    signals: Option<Sender<Signal>>,
    thread: Option<JoinHandle<()>>,
}

impl AutoStopTicker {
    /// Starts ticking `service`.
    pub fn spawn(service: Arc<AutoStopService>) -> Result<Self, DetectError> {
        let (signals, received) = channel();
        let thread = std::thread::Builder::new()
            .name("deino-autostop".to_owned())
            .spawn(move || run(&service, &received))
            .map_err(DetectError::Thread)?;
        Ok(Self {
            signals: Some(signals),
            thread: Some(thread),
        })
    }

    /// The Mac is about to sleep (from the power listener); never blocks.
    pub fn will_sleep(&self) {
        if let Some(signals) = &self.signals {
            signals.send(Signal::WillSleep).ok();
        }
    }
}

/// Ticks until the ticker is dropped.
fn run(service: &AutoStopService, received: &std::sync::mpsc::Receiver<Signal>) {
    loop {
        match received.recv_timeout(TICK) {
            Err(RecvTimeoutError::Timeout) => service.tick(),
            Ok(Signal::WillSleep) => service.will_sleep(),
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

impl Drop for AutoStopTicker {
    fn drop(&mut self) {
        drop(self.signals.take());
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::error!(target: "detect", "the auto-stop thread panicked");
        }
    }
}
