//! `RecordingService`: one recording at a time, start to stop.
//!
//! ```text
//!   Idle ─start / resume_meeting─▶ Starting ─▶ Recording ⇄ Paused
//!                                     │            │         │
//!                                     └─ fails ─▶ Idle       └──stop──▶ Stopping ─▶ Idle
//! ```
//!
//! - **Start:** the session is prepared (engines, writers, capture), then
//!   the meeting row is created (`recording`, `source = live`, its folder,
//!   the Me and Them speakers, the calendar event it was started for), then
//!   transcription starts. The [`StartObserver`] hears of it (the app links
//!   the nearest calendar event, ORG-03), and the meeting ID is returned to
//!   the UI.
//! - **Resume a finished meeting** (CAP-07, [`resume`]): the same, on the
//!   meeting's existing row, folder and speakers; the new audio and segments
//!   follow the recorded length on the same timeline.
//! - **Stop:** capture stops, both tracks close at the same end, the lanes
//!   flush their last segments into `transcript_segments`, the meeting gets
//!   its timing, and it moves to `processing`, which enqueues the
//!   post-meeting pipeline (`finalize_audio` first).
//! - **While recording** ([`controls`]): pause and resume, switch the
//!   microphone, turn system audio off and on, read the levels and what the
//!   lanes heard. A watcher thread keeps the open health issues in the state.
//!
//! The UI never writes the transcript; it watches [`RecordingSnapshot`] and
//! [`crate::events::SegmentUpdate`] events and reads the stored segments.

mod controls;
pub(crate) mod meeting;
mod playhead;
mod resume;

pub use playhead::Playhead;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use deino_core::meetings::{MeetingLink, MeetingTitle};
use deino_domain::{MeetingId, SpeakerId};
use tokio::runtime::Handle;

use crate::deps::{Preferences, RecordingDeps};
use crate::error::RecordingError;
use crate::events::{RecordingPhase, RecordingSnapshot};
use crate::lane::PerLane;
use crate::observer::{NoStartObserver, StartObserver};
use crate::session::{self, Plan, Session, Take};
use crate::state::StateCell;

/// What a new recording is for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartRequest {
    /// The meeting's title; a default when missing or blank.
    pub title: Option<String>,
    /// The calendar event (and series) the meeting is recorded for, if any:
    /// a detected call's event, a split to the next event, a menu-bar Record.
    pub link: Option<MeetingLink>,
}

impl StartRequest {
    /// A recording titled `title`, for no particular event.
    pub fn titled(title: Option<String>) -> Self {
        Self { title, link: None }
    }
}

/// What the service holds between calls.
enum Slot {
    Idle,
    /// Starting or stopping: a call is in flight.
    Busy,
    Active(Box<Running>),
}

/// The running session and the meeting's own start time.
struct Running {
    session: Session,
    /// When the meeting first started (kept by a resumed meeting).
    began_at: DateTime<Utc>,
}

/// The live recorder.
pub struct RecordingService {
    deps: RecordingDeps,
    slot: Mutex<Slot>,
    state: StateCell,
    on_start: Arc<dyn StartObserver>,
}

impl RecordingService {
    /// A service over its ports; nothing is recording.
    pub fn new(deps: RecordingDeps) -> Self {
        let state = StateCell::new(Arc::clone(&deps.events));
        Self {
            deps,
            slot: Mutex::new(Slot::Idle),
            state,
            on_start: Arc::new(NoStartObserver),
        }
    }

    /// The same service, telling `observer` about every recording that starts.
    #[must_use]
    pub fn observing_starts(mut self, observer: Arc<dyn StartObserver>) -> Self {
        self.on_start = observer;
        self
    }

    /// The recorder's state now.
    pub fn state(&self) -> RecordingSnapshot {
        self.state.get()
    }

    /// Starts recording a new meeting; returns its ID.
    pub async fn start(&self, request: StartRequest) -> Result<MeetingId, RecordingError> {
        self.claim()?;
        let titled = request
            .title
            .as_deref()
            .is_some_and(|title| !title.trim().is_empty());
        let started = self.begin(request).await;
        let meeting = self.settle(started)?;
        self.announce_start(&meeting, titled).await;
        Ok(meeting)
    }

    /// Tells the observer, and shows the title it gave the meeting while
    /// that meeting is still the one recording.
    async fn announce_start(&self, meeting: &MeetingId, titled: bool) {
        let Some(title) = self.on_start.started(meeting, titled).await else {
            return;
        };
        self.state.update(|state| {
            if state.meeting_id.as_ref() == Some(meeting) {
                state.title = Some(title);
            }
        });
    }

    /// Prepares the session, creates the meeting, starts transcribing.
    async fn begin(&self, request: StartRequest) -> Result<Launched, RecordingError> {
        let now = self.deps.recorder.wall_clock.now();
        let title = meeting::title(request.title.as_deref())?;
        let plan = self.plan(Take::New, self.deps.preferences.current().await);
        let meeting_id = plan.meeting.clone();
        let row = meeting::MeetingRow {
            meeting: &meeting_id,
            title: &title,
            now,
            speakers: plan.speakers,
            link: request.link.as_ref(),
        };
        let starting = RecordingSnapshot {
            calendar_event_id: request
                .link
                .as_ref()
                .and_then(|link| link.calendar_event_id),
            ..Self::starting(&plan, &title)
        };
        self.state.set(starting.clone());
        let prepared = self.prepare(plan).await?;
        if let Err(error) = meeting::create(&self.deps, &row).await {
            blocking(move || prepared.abort()).await.ok();
            return Err(error);
        }
        self.go(prepared, starting, (now, now)).await
    }

    /// A new meeting's ID, speakers and folder.
    fn plan(&self, take: Take, preferences: Preferences) -> Plan {
        let clock = self.deps.recorder.wall_clock.as_ref();
        let meeting = meeting::new_id(clock);
        Plan {
            dir: deino_audio::recording_dir(&self.deps.recordings, &meeting),
            meeting,
            speakers: PerLane {
                me: Some(SpeakerId::generate(clock)),
                them: Some(SpeakerId::generate(clock)),
            },
            preferences,
            runtime: Handle::current(),
            take,
        }
    }

    /// The state while `plan` starts.
    fn starting(plan: &Plan, title: &MeetingTitle) -> RecordingSnapshot {
        RecordingSnapshot {
            phase: RecordingPhase::Starting,
            meeting_id: Some(plan.meeting.clone()),
            title: Some(title.as_str().to_owned()),
            microphone: plan.preferences.microphone.preferred.clone(),
            ..RecordingSnapshot::idle()
        }
    }

    /// Opens the engines, writers and capture, off the async runtime.
    async fn prepare(&self, plan: Plan) -> Result<session::Prepared, RecordingError> {
        let deps = self.deps.clone();
        blocking(move || session::prepare(&deps, plan)).await?
    }

    /// Starts transcribing; the recording's clock counts from `clock_from`.
    async fn go(
        &self,
        prepared: session::Prepared,
        starting: RecordingSnapshot,
        (began_at, clock_from): (DateTime<Utc>, DateTime<Utc>),
    ) -> Result<Launched, RecordingError> {
        let (deps, state) = (self.deps.clone(), self.state.clone());
        let session = blocking(move || prepared.go(&deps, state)).await??;
        let recording = RecordingSnapshot {
            phase: RecordingPhase::Recording,
            started_at: Some(clock_from),
            ..starting
        };
        Ok(Launched {
            running: Running { session, began_at },
            state: recording,
        })
    }

    /// Installs a launched session, or returns to idle after a failure.
    fn settle(
        &self,
        launched: Result<Launched, RecordingError>,
    ) -> Result<MeetingId, RecordingError> {
        match launched {
            Ok(Launched { running, state }) => {
                let meeting = running.session.meeting().clone();
                *lock(&self.slot) = Slot::Active(Box::new(running));
                self.state.set(state);
                Ok(meeting)
            }
            Err(error) => {
                *lock(&self.slot) = Slot::Idle;
                self.state.set(RecordingSnapshot::idle());
                Err(error)
            }
        }
    }

    /// Stops the recording, saves its last segments and moves the meeting
    /// to `processing`; returns its ID.
    pub async fn stop(&self) -> Result<MeetingId, RecordingError> {
        let running = {
            let mut slot = lock(&self.slot);
            match std::mem::replace(&mut *slot, Slot::Busy) {
                Slot::Active(running) => running,
                other => {
                    *slot = other;
                    return Err(RecordingError::NotRecording);
                }
            }
        };
        self.state
            .update(|state| state.phase = RecordingPhase::Stopping);
        let Running { session, began_at } = *running;
        let meeting = session.meeting().clone();
        let host_clock = std::sync::Arc::clone(&self.deps.recorder.host_clock);
        let stopped = blocking(move || session.stop(host_clock.as_ref())).await;
        *lock(&self.slot) = Slot::Idle;
        self.state.set(RecordingSnapshot::idle());
        let length = stopped??;
        let ended_at = self.deps.recorder.wall_clock.now();
        meeting::close(&self.deps, &meeting, (began_at, ended_at), length).await?;
        Ok(meeting)
    }

    /// Takes the slot for a start, or fails when a recording runs.
    fn claim(&self) -> Result<(), RecordingError> {
        let mut slot = lock(&self.slot);
        if !matches!(*slot, Slot::Idle) {
            return Err(RecordingError::AlreadyRecording);
        }
        *slot = Slot::Busy;
        Ok(())
    }

    /// Runs `act` on the running session, or fails when nothing records.
    fn with_session<T>(&self, act: impl FnOnce(&Session) -> T) -> Result<T, RecordingError> {
        match &*lock(&self.slot) {
            Slot::Active(running) => Ok(act(&running.session)),
            Slot::Idle | Slot::Busy => Err(RecordingError::NotRecording),
        }
    }
}

/// A session that started, and the state to publish for it.
struct Launched {
    running: Running,
    state: RecordingSnapshot,
}

/// Runs blocking work (device and engine startup, joins) off the async runtime.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, RecordingError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| RecordingError::Thread(std::io::Error::other(error)))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
