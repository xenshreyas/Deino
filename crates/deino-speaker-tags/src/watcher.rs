//! [`Watcher`]: a [`Watch`] on its own thread, polling every interval
//! until the recording ends or the watcher is stopped.
//!
//! The thread waits on its stop channel with a timeout, so the timeout is
//! the poll timer and a stop (or a dropped [`Watcher`]) wakes it at once.
//! Nothing here touches audio: reading other apps' windows takes
//! milliseconds and may block on a slow app, so it never runs on the
//! capture, encoder or transcription threads.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::watch::{Step, Watch};

/// A running watch.
#[derive(Debug)]
pub struct Watcher {
    stop: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Watcher {
    /// Starts polling `watch` every `interval` on a new thread.
    pub fn spawn(watch: Watch, interval: Duration) -> std::io::Result<Self> {
        let (stop, stopped) = channel();
        let thread = std::thread::Builder::new()
            .name("deino-speaker-tags".into())
            .spawn(move || run(watch, &stopped, interval))?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }

    /// Stops polling, saves what was seen, and waits for the thread.
    pub fn stop(mut self) {
        // The thread may already be gone (the recording ended); then there
        // is nobody to tell.
        self.stop.send(()).ok();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }

    /// Whether the thread has finished (the recording ended).
    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

/// Polls until the recording ends or a stop arrives.
fn run(mut watch: Watch, stopped: &Receiver<()>, interval: Duration) {
    loop {
        match stopped.recv_timeout(interval) {
            Err(RecvTimeoutError::Timeout) => {
                if watch.tick() == Step::Done {
                    return;
                }
            }
            Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                watch.finish();
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use deino_domain::{MeetingId, Millis};

    use super::*;
    use crate::fake::{FakeMac, MemorySink, ScriptedClock};
    use crate::ports::Position;
    use crate::status::LayoutHealth;
    use crate::watch::WatchParts;

    fn watch(clock: Arc<ScriptedClock>, sink: Arc<MemorySink>) -> Watch {
        Watch::new(WatchParts {
            meeting: MeetingId::new("meeting"),
            platform: FakeMac::granted().platform(),
            clock,
            sink,
            health: Arc::new(LayoutHealth::default()),
        })
    }

    #[test]
    fn the_thread_ends_by_itself_when_the_recording_ends() {
        let clock = ScriptedClock::new([Position::At(Millis(0)), Position::At(Millis(250))]);
        let interval = Duration::from_millis(1);
        let mut watcher = Watcher::spawn(watch(clock, Arc::default()), interval).unwrap();

        // The stop sender is still alive: only the clock's `Over` ends it.
        watcher.thread.take().unwrap().join().unwrap();

        assert!(watcher.is_finished());
    }

    #[test]
    fn stopping_wakes_the_thread_at_once() {
        let positions = std::iter::repeat_n(Position::Paused, 1_000);
        let clock = ScriptedClock::new(positions);
        let watcher =
            Watcher::spawn(watch(clock, Arc::default()), Duration::from_hours(1)).unwrap();

        watcher.stop();
    }
}
