//! [`Runs`]: the meetings whose notes are being written, each with its
//! latest draft and a way to stop it.
//!
//! One run per meeting at a time ([`EnhanceError::AlreadyRunning`]). A
//! [`Run`] guard publishes the run's drafts, keeps the latest for a page
//! opened mid-run ([`Runs::draft`]), and on drop forgets the run and
//! reports how it ended, once it reported anything. [`Runs::stop`] aborts
//! the run's writing: the calls in flight are dropped and nothing is saved.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deino_domain::MeetingId;
use deino_llm::LlmError;
use futures_util::future::{AbortHandle, AbortRegistration, Abortable, Aborted};

use super::EnhanceError;
use super::progress::{NotesDraft, NotesProgress, NotesRunEnd};

/// A running meeting's stop handle and latest draft.
struct Slot {
    abort: AbortHandle,
    draft: Option<NotesDraft>,
}

/// The runs in progress.
pub(super) struct Runs {
    slots: Mutex<HashMap<MeetingId, Slot>>,
    progress: Arc<dyn NotesProgress>,
}

impl Runs {
    /// No runs, reporting to `progress`.
    pub(super) fn new(progress: Arc<dyn NotesProgress>) -> Self {
        Self {
            slots: Mutex::new(HashMap::new()),
            progress,
        }
    }

    /// Starts a run for `meeting`, or fails when one is in progress.
    pub(super) fn start(&self, meeting: &MeetingId) -> Result<Run<'_>, EnhanceError> {
        let mut slots = self.lock();
        if slots.contains_key(meeting) {
            return Err(EnhanceError::AlreadyRunning);
        }
        let (abort, registration) = AbortHandle::new_pair();
        slots.insert(meeting.clone(), Slot { abort, draft: None });
        Ok(Run {
            runs: self,
            meeting: meeting.clone(),
            registration: Mutex::new(Some(registration)),
            state: Mutex::new(RunState {
                published: false,
                kept: 0,
                end: NotesRunEnd::Failed,
            }),
        })
    }

    /// The latest draft of `meeting`'s run, if one is in progress.
    pub(super) fn draft(&self, meeting: &MeetingId) -> Option<NotesDraft> {
        self.lock().get(meeting).and_then(|slot| slot.draft.clone())
    }

    /// Stops `meeting`'s run; false when none is in progress.
    pub(super) fn stop(&self, meeting: &MeetingId) -> bool {
        let slots = self.lock();
        let Some(slot) = slots.get(meeting) else {
            return false;
        };
        slot.abort.abort();
        log::info!(target: "enhance", "{meeting}: writing the notes was stopped");
        true
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<MeetingId, Slot>> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What a run has reported, and how it ends.
struct RunState {
    published: bool,
    /// The lines the user typed, which the saved notes keep.
    kept: u32,
    end: NotesRunEnd,
}

/// While alive, its meeting's notes count as being written.
pub(super) struct Run<'a> {
    runs: &'a Runs,
    meeting: MeetingId,
    registration: Mutex<Option<AbortRegistration>>,
    state: Mutex<RunState>,
}

impl Run<'_> {
    /// Keeps `draft` as the run's latest and reports it.
    pub(super) fn publish(&self, draft: NotesDraft) {
        self.state().published = true;
        self.runs.progress.draft(&draft);
        if let Some(slot) = self.runs.lock().get_mut(&self.meeting) {
            slot.draft = Some(draft);
        }
    }

    /// Runs the writing `work` until it ends or the run is stopped
    /// ([`EnhanceError::Stopped`]). Only the writing: saving the version
    /// afterwards is never cut short.
    pub(super) async fn abortable<T>(
        &self,
        work: impl Future<Output = Result<T, LlmError>>,
    ) -> Result<T, EnhanceError> {
        let registration = self
            .registration
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        let Some(registration) = registration else {
            return Ok(work.await?);
        };
        match Abortable::new(work, registration).await {
            Ok(written) => Ok(written?),
            Err(Aborted) => Err(EnhanceError::Stopped),
        }
    }

    /// Records how many lines the user typed (the saved notes keep each).
    pub(super) fn keeps(&self, lines: usize) {
        self.state().kept = u32::try_from(lines).unwrap_or(u32::MAX);
    }

    /// Records how the run ended, from its result.
    pub(super) fn finish(&self, result: &Result<u32, EnhanceError>) {
        let mut state = self.state();
        state.end = match result {
            Ok(version) => NotesRunEnd::Written {
                version: *version,
                kept: state.kept,
            },
            Err(EnhanceError::Stopped) => NotesRunEnd::Stopped,
            Err(_) => NotesRunEnd::Failed,
        };
    }

    /// The meeting being written.
    pub(super) fn meeting(&self) -> &MeetingId {
        &self.meeting
    }

    fn state(&self) -> MutexGuard<'_, RunState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for Run<'_> {
    fn drop(&mut self) {
        self.runs.lock().remove(&self.meeting);
        let state = self.state();
        if state.published {
            self.runs.progress.ended(&self.meeting, state.end);
        }
    }
}
