//! `RecordingService`: one recording at a time, start to stop.
//!
//! ```text
//!   Idle ─start─▶ Starting ─▶ Recording ⇄ Paused
//!                    │            │         │
//!                    └─ fails ─▶ Idle       └──stop──▶ Stopping ─▶ Idle
//! ```
//!
//! - **Start:** the session is prepared (engines, writers, capture), then
//!   the meeting row is created (`recording`, `source = live`, its folder,
//!   the Me and Them speakers), then transcription starts. The
//!   [`StartObserver`] hears of it (the app links the nearest calendar
//!   event, ORG-03), and the meeting ID is returned to the UI.
//! - **Stop:** capture stops, both tracks close at the same end, the lanes
//!   flush their last segments into `transcript_segments`, the meeting gets
//!   its timing, and it moves to `processing`, which enqueues the
//!   post-meeting pipeline (`finalize_audio` first).
//!
//! The UI never writes the transcript; it watches [`RecordingSnapshot`] and
//! [`crate::events::SegmentUpdate`] events and reads the stored segments.

pub(crate) mod meeting;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use deino_core::meetings::MeetingTitle;
use deino_domain::{MeetingId, SpeakerId};
use tokio::runtime::Handle;

use crate::deps::{Preferences, RecordingDeps};
use crate::error::RecordingError;
use crate::events::{RecordingPhase, RecordingSnapshot};
use crate::lane::PerLane;
use crate::observer::{NoStartObserver, StartObserver};
use crate::session::{self, Plan, Session};

/// What the service holds between calls.
enum Slot {
    Idle,
    /// Starting or stopping: a call is in flight.
    Busy,
    Active(Box<Session>),
}

/// The live recorder.
pub struct RecordingService {
    deps: RecordingDeps,
    slot: Mutex<Slot>,
    state: Mutex<RecordingSnapshot>,
    on_start: Arc<dyn StartObserver>,
}

impl RecordingService {
    /// A service over its ports; nothing is recording.
    pub fn new(deps: RecordingDeps) -> Self {
        Self {
            deps,
            slot: Mutex::new(Slot::Idle),
            state: Mutex::new(RecordingSnapshot::idle()),
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
        lock(&self.state).clone()
    }

    /// Starts recording a new meeting titled `title` (a default when
    /// missing or blank); returns its ID.
    pub async fn start(&self, title: Option<String>) -> Result<MeetingId, RecordingError> {
        self.claim()?;
        let titled = title
            .as_deref()
            .is_some_and(|title| !title.trim().is_empty());
        let result = self.begin(title).await;
        match result {
            Ok((session, state)) => {
                let meeting = session.meeting().clone();
                *lock(&self.slot) = Slot::Active(Box::new(session));
                self.publish(state);
                self.announce_start(&meeting, titled).await;
                Ok(meeting)
            }
            Err(error) => {
                *lock(&self.slot) = Slot::Idle;
                self.publish(RecordingSnapshot::idle());
                Err(error)
            }
        }
    }

    /// Prepares the session, creates the meeting, starts transcribing.
    async fn begin(
        &self,
        title: Option<String>,
    ) -> Result<(Session, RecordingSnapshot), RecordingError> {
        let now = self.deps.recorder.wall_clock.now();
        let title = meeting::title(title.as_deref())?;
        let plan = self.plan(self.deps.preferences.current().await);
        let (meeting, speakers) = (plan.meeting.clone(), plan.speakers);
        self.publish(state(RecordingPhase::Starting, &meeting, &title));
        let deps = self.deps.clone();
        let prepared = blocking(move || session::prepare(&deps, plan)).await??;
        let row = meeting::MeetingRow {
            meeting: &meeting,
            title: &title,
            now,
            speakers,
        };
        if let Err(error) = meeting::create(&self.deps, &row).await {
            blocking(move || prepared.abort()).await.ok();
            return Err(error);
        }
        let events = Arc::clone(&self.deps.events);
        let session = blocking(move || prepared.go(events)).await??;
        let recording = RecordingSnapshot {
            started_at: Some(now),
            ..state(RecordingPhase::Recording, &meeting, &title)
        };
        Ok((session, recording))
    }

    /// Tells the observer, and shows the title it gave the meeting while
    /// that meeting is still the one recording.
    async fn announce_start(&self, meeting: &MeetingId, titled: bool) {
        let Some(title) = self.on_start.started(meeting, titled).await else {
            return;
        };
        self.update(|state| {
            if state.meeting_id.as_ref() == Some(meeting) {
                state.title = Some(title);
            }
        });
    }

    /// A new meeting's ID, speakers and folder.
    fn plan(&self, preferences: Preferences) -> Plan {
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
        }
    }

    /// Pauses capture; the recording's timeline skips the pause.
    pub fn pause(&self) -> Result<RecordingSnapshot, RecordingError> {
        let slot = lock(&self.slot);
        let (Slot::Active(session), RecordingPhase::Recording) = (&*slot, self.state().phase)
        else {
            return Err(RecordingError::NotRecording);
        };
        session.pause();
        let now = self.deps.recorder.wall_clock.now();
        Ok(self.update(|state| {
            state.phase = RecordingPhase::Paused;
            state.paused_at = Some(now);
        }))
    }

    /// Resumes a paused recording.
    pub fn resume(&self) -> Result<RecordingSnapshot, RecordingError> {
        let slot = lock(&self.slot);
        let (Slot::Active(session), RecordingPhase::Paused) = (&*slot, self.state().phase) else {
            return Err(RecordingError::NotRecording);
        };
        session.resume();
        let now = self.deps.recorder.wall_clock.now();
        Ok(self.update(|state| {
            let paused = state
                .paused_at
                .take()
                .map(|at| paused_ms(at, now))
                .unwrap_or_default();
            state.paused_ms += paused;
            state.phase = RecordingPhase::Recording;
        }))
    }

    /// Stops the recording, saves its last segments and moves the meeting
    /// to `processing`; returns its ID.
    pub async fn stop(&self) -> Result<MeetingId, RecordingError> {
        let session = {
            let mut slot = lock(&self.slot);
            match std::mem::replace(&mut *slot, Slot::Busy) {
                Slot::Active(session) => session,
                other => {
                    *slot = other;
                    return Err(RecordingError::NotRecording);
                }
            }
        };
        let started_at = self
            .update(|state| state.phase = RecordingPhase::Stopping)
            .started_at;
        let meeting = session.meeting().clone();
        let host_clock = Arc::clone(&self.deps.recorder.host_clock);
        let stopped = blocking(move || session.stop(host_clock.as_ref())).await;
        *lock(&self.slot) = Slot::Idle;
        self.publish(RecordingSnapshot::idle());
        let length = stopped??;
        let ended_at = self.deps.recorder.wall_clock.now();
        meeting::close(
            &self.deps,
            &meeting,
            (started_at.unwrap_or(ended_at), ended_at),
            length,
        )
        .await?;
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

    fn update(&self, change: impl FnOnce(&mut RecordingSnapshot)) -> RecordingSnapshot {
        let state = {
            let mut state = lock(&self.state);
            change(&mut state);
            state.clone()
        };
        self.deps.events.state(&state);
        state
    }

    fn publish(&self, state: RecordingSnapshot) {
        self.update(|current| *current = state);
    }
}

/// The state of `meeting` in `phase`.
fn state(phase: RecordingPhase, meeting: &MeetingId, title: &MeetingTitle) -> RecordingSnapshot {
    RecordingSnapshot {
        phase,
        meeting_id: Some(meeting.clone()),
        title: Some(title.as_str().to_owned()),
        ..RecordingSnapshot::idle()
    }
}

/// Runs blocking work (device and engine startup, joins) off the async runtime.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, RecordingError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| RecordingError::Thread(std::io::Error::other(error)))
}

fn paused_ms(from: DateTime<Utc>, to: DateTime<Utc>) -> u64 {
    u64::try_from((to - from).num_milliseconds()).unwrap_or_default()
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
