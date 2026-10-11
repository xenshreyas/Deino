//! The session's health watcher: a thread that gathers [`Vitals`] once a
//! [`TICK`], runs the [`HealthMonitor`] and publishes the open issues in
//! the recorder's state. It only reads atomics and the disk's free space;
//! it never touches audio.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_audio::capture::TrackStates;
use deino_audio::{DiskSpace, HostClock};
use deino_domain::MeetingId;
use deino_stt::StatusHandle;

use super::{HealthMonitor, TrackVitals, Vitals};
use crate::activity::LiveActivity;
use crate::lane::{Lane, PerLane};
use crate::state::StateCell;

/// How often the watcher looks.
pub const TICK: Duration = Duration::from_secs(1);

/// What a watcher reads and where it publishes.
pub(crate) struct WatchParts {
    /// The meeting being recorded.
    pub meeting: MeetingId,
    /// Each track's meter and status.
    pub tracks: Arc<TrackStates>,
    /// Each lane's speech and text.
    pub activity: Arc<LiveActivity>,
    /// Each lane's engine status, for engines that stream.
    pub engines: PerLane<Option<StatusHandle>>,
    /// Free space.
    pub disk: Arc<dyn DiskSpace>,
    /// The recording's folder (its volume is the one that fills).
    pub dir: PathBuf,
    /// Time.
    pub clock: Arc<dyn HostClock>,
    /// The recorder's state, where the open issues go.
    pub state: StateCell,
}

/// A running watcher; dropping it (or [`Watcher::stop`]) ends the thread.
pub(crate) struct Watcher {
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Watcher {
    /// Starts watching.
    pub fn start(parts: WatchParts) -> std::io::Result<Self> {
        let (stop, stopped) = channel::<()>();
        let thread = std::thread::Builder::new()
            .name("deino-recording-health".into())
            .spawn(move || {
                let mut monitor = HealthMonitor::new(parts.clock.now());
                while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(TICK) {
                    look(&parts, &mut monitor);
                }
            })?;
        Ok(Self {
            stop: Some(stop),
            thread: Some(thread),
        })
    }

    /// Stops the thread and waits for it.
    pub fn stop(mut self) {
        self.shut_down();
    }

    fn shut_down(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::error!(target: "recording", "the health watcher panicked");
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.shut_down();
    }
}

/// One look: gather, judge, publish what changed.
fn look(parts: &WatchParts, monitor: &mut HealthMonitor) {
    let vitals = gather(parts);
    if !monitor.evaluate(&vitals, parts.clock.now()) {
        return;
    }
    let issues = monitor.issues().to_vec();
    log::info!(target: "recording", "recording health: {issues:?}");
    parts.state.update(|state| {
        if state.meeting_id.as_ref() == Some(&parts.meeting) {
            state.issues = issues;
        }
    });
}

/// The vitals now.
fn gather(parts: &WatchParts) -> Vitals {
    let track = |lane: Lane| {
        let state = parts.tracks.get(lane.track());
        TrackVitals {
            frames: state.meter().frames(),
            status: state.status(),
        }
    };
    let engine = |lane: Lane| parts.engines.get(lane).as_ref().map(StatusHandle::read);
    Vitals {
        tracks: PerLane {
            me: track(Lane::Me),
            them: track(Lane::Them),
        },
        unanswered: parts.activity.read(Lane::Me).unanswered,
        engines: PerLane {
            me: engine(Lane::Me),
            them: engine(Lane::Them),
        },
        free_disk: parts.disk.free(&parts.dir).ok(),
    }
}
