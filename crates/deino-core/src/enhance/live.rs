//! [`LiveDraft`]: one run's notes as they are written (streamed notes,
//! ADR 2026-10-30-streamed-notes).
//!
//! It is the [`AnswerSink`] of the calls whose answer becomes the notes
//! (one pass, or the merge) and the [`WriteReport`] of the writer. Each
//! piece of the answer is added to the text so far; when the answer's
//! finished lines ([`complete_lines`]) grow, they are checked like the
//! saved notes ([`preview`]) and published as a [`NotesDraft`]. A new
//! attempt (a retry, the repair turn, a merge after a failed one pass)
//! keeps the old lines, marked stale, until its own first line arrives.

use std::sync::{Mutex, MutexGuard, PoisonError};

use deino_doc::Doc;
use deino_llm::AnswerSink;

use super::assemble::{Checks, preview};
use super::convert::to_doc;
use super::partial::{complete_lines, line_count};
use super::progress::{NotesDraft, NotesPhase};
use super::runs::Run;
use super::writer::WriteReport;

/// What the run shows now.
struct State {
    /// `Reading` or `Merging`: what the current call does.
    base: NotesPhase,
    phase: NotesPhase,
    /// The current attempt's answer so far.
    text: String,
    /// Finished lines in the current attempt's answer.
    attempt_lines: usize,
    doc: Doc,
    lines: usize,
    stale: bool,
}

/// One run's draft (see the module docs).
pub(super) struct LiveDraft<'a> {
    run: &'a Run<'a>,
    checks: Checks<'a>,
    state: Mutex<State>,
}

impl<'a> LiveDraft<'a> {
    /// A run that starts reading; publishes that at once.
    pub(super) fn new(run: &'a Run<'a>, checks: Checks<'a>) -> Self {
        let live = Self {
            run,
            checks,
            state: Mutex::new(State {
                base: NotesPhase::Reading,
                phase: NotesPhase::Reading,
                text: String::new(),
                attempt_lines: 0,
                doc: Doc::empty(),
                lines: 0,
                stale: false,
            }),
        };
        live.publish(&live.lock());
        live
    }

    /// The notes are written; they are being saved.
    pub(super) fn saving(&self) {
        let mut state = self.lock();
        state.phase = NotesPhase::Saving;
        self.publish(&state);
    }

    fn publish(&self, state: &State) {
        self.run.publish(NotesDraft {
            meeting_id: self.run.meeting().clone(),
            phase: state.phase,
            doc: state.doc.clone(),
            lines: u32::try_from(state.lines).unwrap_or(u32::MAX),
            stale: state.stale,
        });
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl AnswerSink for LiveDraft<'_> {
    fn start(&self) {
        let mut state = self.lock();
        let again = !state.text.is_empty();
        state.text.clear();
        state.attempt_lines = 0;
        if again {
            state.stale = state.lines > 0;
            state.phase = NotesPhase::Retrying;
            self.publish(&state);
        }
    }

    fn piece(&self, piece: &str) {
        let mut state = self.lock();
        state.text.push_str(piece);
        let answer = complete_lines(&state.text);
        let lines = line_count(&answer);
        if lines == state.attempt_lines {
            return;
        }
        state.attempt_lines = lines;
        state.doc = to_doc(&preview(&answer, self.checks));
        state.lines = lines;
        state.stale = false;
        state.phase = state.base;
        self.publish(&state);
    }
}

impl WriteReport for LiveDraft<'_> {
    fn sink(&self) -> &dyn AnswerSink {
        self
    }

    fn parts(&self, done: usize, total: usize) {
        let mut state = self.lock();
        state.stale = state.lines > 0;
        state.phase = NotesPhase::Parts {
            done: u32::try_from(done).unwrap_or(u32::MAX),
            total: u32::try_from(total).unwrap_or(u32::MAX),
        };
        self.publish(&state);
    }

    fn merging(&self) {
        let mut state = self.lock();
        state.base = NotesPhase::Merging;
        state.phase = NotesPhase::Merging;
        state.text.clear();
        state.attempt_lines = 0;
        self.publish(&state);
    }
}
