//! [`Monitor`]: follows the recorder and runs one [`Session`] per recorded
//! meeting; [`ChatNotice`]: the monitor on its own thread, polling every
//! [`POLL_INTERVAL`] for as long as the app runs.
//!
//! The thread waits on its stop channel with a timeout, so the timeout is
//! the poll timer and dropping the [`ChatNotice`] wakes it at once. A poll
//! while nothing records, or with the notice off, reads one setting and
//! the recorder's state and returns. Nothing here touches audio.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_domain::MeetingId;

use crate::error::{ChatError, LoggedOnce};
use crate::ports::{ConsentPlatform, NoticeConfig, Recorder, RecorderNow};
use crate::session::{Ports, Progress, Session};

/// How often the monitor polls.
pub const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// What a [`Monitor`] is built over.
#[derive(Clone)]
pub struct NoticeParts {
    /// This Mac's ports; `None` where there is no Accessibility.
    pub platform: Option<ConsentPlatform>,
    /// The recorder.
    pub recorder: Arc<dyn Recorder>,
    /// The user's settings.
    pub config: Arc<dyn NoticeConfig>,
}

/// One session per recorded meeting, and the meetings already done.
pub struct Monitor {
    parts: NoticeParts,
    session: Option<Session>,
    finished: HashMap<MeetingId, Progress>,
    logged: LoggedOnce,
}

impl Monitor {
    /// A monitor over `parts`; nothing recorded yet.
    pub fn new(parts: NoticeParts) -> Self {
        Self {
            parts,
            session: None,
            finished: HashMap::new(),
            logged: LoggedOnce::default(),
        }
    }

    /// Where `meeting`'s notice stands, if it was ever looked at.
    pub fn progress(&self, meeting: &MeetingId) -> Option<Progress> {
        match &self.session {
            Some(session) if session.meeting() == meeting => Some(session.progress()),
            Some(_) | None => self.finished.get(meeting).copied(),
        }
    }

    /// One poll.
    pub fn tick(&mut self) {
        let (meeting, transcribing) = match self.parts.recorder.now() {
            RecorderNow::Recording(meeting) => (meeting, true),
            RecorderNow::Paused(meeting) => (meeting, false),
            RecorderNow::Idle => return self.close(),
        };
        if !self.follow(meeting) {
            return;
        }
        let config = self.parts.config.chat();
        let Some(platform) = &self.parts.platform else {
            if config.is_on() {
                self.logged.log(&ChatError::Unsupported);
            }
            return;
        };
        let Some(session) = &mut self.session else {
            return;
        };
        let ports = Ports {
            platform,
            recorder: self.parts.recorder.as_ref(),
        };
        let progress = session.tick(ports, &config, transcribing);
        if progress.is_final() {
            self.finished.insert(session.meeting().clone(), progress);
        }
    }

    /// Makes `meeting` the current session; false when its notice is
    /// already done (posted or given up), so it is never posted twice.
    fn follow(&mut self, meeting: MeetingId) -> bool {
        if self
            .session
            .as_ref()
            .is_some_and(|session| *session.meeting() == meeting)
        {
            return true;
        }
        self.close();
        if self.finished.contains_key(&meeting) {
            return false;
        }
        self.session = Some(Session::new(meeting));
        true
    }

    /// Ends the current session, remembering a finished one.
    fn close(&mut self) {
        if let Some(session) = self.session.take()
            && session.progress().is_final()
        {
            self.finished
                .insert(session.meeting().clone(), session.progress());
        }
    }
}

/// The running monitor (see the module docs).
#[derive(Debug)]
pub struct ChatNotice {
    stop: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl ChatNotice {
    /// Starts polling `monitor` every `interval` on a new thread.
    pub fn spawn(monitor: Monitor, interval: Duration) -> std::io::Result<Self> {
        let (stop, stopped) = channel();
        let thread = std::thread::Builder::new()
            .name("deino-consent".into())
            .spawn(move || run(monitor, &stopped, interval))?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }

    /// Stops polling and waits for the thread.
    pub fn stop(mut self) {
        self.stop.send(()).ok();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

/// Polls until a stop arrives or the handle is dropped.
fn run(mut monitor: Monitor, stopped: &Receiver<()>, interval: Duration) {
    while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(interval) {
        monitor.tick();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{FakeRecorder, FixedConfig};
    use crate::ports::ChatConfig;

    #[test]
    fn the_thread_polls_until_stopped() {
        let recorder = FakeRecorder::recording("meeting", false);
        recorder.set(RecorderNow::Idle);
        let monitor = Monitor::new(NoticeParts {
            platform: None,
            recorder,
            config: FixedConfig::new(ChatConfig::default()),
        });
        let notice = ChatNotice::spawn(monitor, Duration::from_millis(1)).unwrap();

        notice.stop();
    }
}
