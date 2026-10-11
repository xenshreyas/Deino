//! The router thread: lane text in, saved segments and live events out.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel, sync_channel};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_audio::{HostClock, HostTime};
use deino_core::segments::NewSegment;
use deino_domain::{Clock, MeetingId, SegmentId, SpeakerId};
use deino_stt::{FinalSegment, LiveText};

use super::drafts::{Drafts, Owner};
use super::{RouterHandle, RouterInput, SegmentStore};
use crate::bleed::{Holdback, HoldbackPolicy, Offer, Verdict};
use crate::echo::EchoMode;
use crate::events::{RecordingEvents, SegmentUpdate, SegmentUpdateKind};
use crate::lane::{Lane, PerLane};
use crate::vad::VoiceChange;

/// How often held segments are checked.
pub const TICK: Duration = Duration::from_millis(250);

/// How often drafts and finals are saved: the most transcript `kill -9` can
/// lose beyond the engine's own latency.
pub const FLUSH: Duration = Duration::from_secs(1);

/// What the router is built from.
pub struct RouterParts {
    /// The meeting being recorded.
    pub meeting: MeetingId,
    /// Each lane's speaker row (Me, and "Them" until diarization).
    pub speakers: PerLane<Option<SpeakerId>>,
    /// Where segments are saved.
    pub store: Arc<dyn SegmentStore>,
    /// Where live segments are shown.
    pub events: Arc<dyn RecordingEvents>,
    /// Wall time, for segment IDs.
    pub clock: Arc<dyn Clock>,
    /// Monotonic time, for holds and flushes.
    pub host_clock: Arc<dyn HostClock>,
    /// The holdback's thresholds.
    pub policy: HoldbackPolicy,
}

/// A held Me segment and whether its draft row exists.
#[derive(Debug)]
struct Pending {
    row: NewSegment,
    draft_saved: bool,
}

/// The running router.
pub struct Router {
    handle: RouterHandle,
    thread: Option<JoinHandle<()>>,
}

impl Router {
    /// Starts the router thread.
    pub fn start(parts: RouterParts) -> std::io::Result<Self> {
        let (inputs, receiver) = channel();
        let mut state = State::new(parts);
        let thread = std::thread::Builder::new()
            .name("deino-transcript-router".into())
            .spawn(move || state.run(&receiver))?;
        Ok(Self {
            handle: RouterHandle { inputs },
            thread: Some(thread),
        })
    }

    /// The handle lanes send through.
    pub fn handle(&self) -> RouterHandle {
        self.handle.clone()
    }

    /// Judges every held segment, saves everything and stops. Call it after
    /// the lanes have finished.
    pub fn finish(mut self) {
        let (done, finished) = sync_channel(1);
        self.handle.send(RouterInput::Finish(done));
        finished.recv().ok();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::error!(target: "recording", "the transcript router panicked");
        }
    }
}

/// The router thread's state.
struct State {
    owner: Owner,
    store: Arc<dyn SegmentStore>,
    events: Arc<dyn RecordingEvents>,
    clock: Arc<dyn Clock>,
    host_clock: Arc<dyn HostClock>,
    holdback: Holdback<Pending>,
    drafts: Drafts,
    mode: EchoMode,
    them_speaking: bool,
    writes: Vec<NewSegment>,
    removed: Vec<SegmentId>,
    flushed_at: HostTime,
}

impl State {
    fn new(parts: RouterParts) -> Self {
        Self {
            owner: Owner {
                meeting: parts.meeting,
                speakers: parts.speakers,
            },
            store: parts.store,
            events: parts.events,
            clock: parts.clock,
            flushed_at: parts.host_clock.now(),
            host_clock: parts.host_clock,
            holdback: Holdback::new(parts.policy),
            drafts: Drafts::default(),
            mode: EchoMode::Cancel,
            them_speaking: false,
            writes: Vec::new(),
            removed: Vec::new(),
        }
    }

    fn run(&mut self, inputs: &Receiver<RouterInput>) {
        loop {
            match inputs.recv_timeout(TICK) {
                Ok(RouterInput::Finish(done)) => {
                    self.finish();
                    done.send(()).ok();
                    return;
                }
                Ok(input) => self.handle(input),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return self.finish(),
            }
            self.tick();
        }
    }

    fn handle(&mut self, input: RouterInput) {
        match input {
            RouterInput::Live(lane, text) => self.live(lane, &text),
            RouterInput::Final(lane, segment) => self.closed(lane, &segment),
            RouterInput::Voice(lane, change) => self.voice(lane, change),
            RouterInput::Mode(mode) => self.mode = mode,
            RouterInput::Finish(_) => {}
        }
    }

    /// A lane's open segment grew.
    fn live(&mut self, lane: Lane, text: &LiveText) {
        let clock = Arc::clone(&self.clock);
        let Some(draft) = self
            .drafts
            .update(lane, text, || SegmentId::generate(clock.as_ref()))
        else {
            return;
        };
        let row = self.owner.draft_row(lane, draft);
        self.emit(lane, &row, SegmentUpdateKind::Open);
        if lane == Lane::Them {
            let verdicts = self.holdback.them_live(text.start, &text.committed);
            self.apply(verdicts);
        }
    }

    /// A lane closed a segment.
    fn closed(&mut self, lane: Lane, segment: &FinalSegment) {
        let draft = self.drafts.close(lane, segment.start);
        let draft_saved = draft.as_ref().is_some_and(|draft| draft.saved);
        let id = draft.map_or_else(
            || SegmentId::generate(self.clock.as_ref()),
            |draft| draft.id,
        );
        let row = self.owner.final_row(lane, id, segment);
        if lane == Lane::Them || self.mode == EchoMode::Isolated {
            if lane == Lane::Them {
                let verdicts = self
                    .holdback
                    .them_final(segment.start, segment.end, &segment.text);
                self.apply(verdicts);
            }
            return self.keep(row);
        }
        let offer = Offer {
            start: segment.start,
            end: segment.end,
            text: segment.text.clone(),
            payload: Pending { row, draft_saved },
        };
        let verdict = self.holdback.offer(offer, self.host_clock.now());
        self.apply(verdict.into_iter().collect());
    }

    fn voice(&mut self, lane: Lane, change: VoiceChange) {
        if lane != Lane::Them {
            return;
        }
        match change {
            VoiceChange::Started(_) => self.them_speaking = true,
            VoiceChange::Ended(span) => {
                self.them_speaking = false;
                self.holdback.them_voice(span);
            }
        }
    }

    fn apply(&mut self, verdicts: Vec<Verdict<Pending>>) {
        for verdict in verdicts {
            match verdict {
                Verdict::Keep(pending) => self.keep(pending.row),
                Verdict::Drop(pending) => self.discard(&pending),
            }
        }
    }

    fn keep(&mut self, row: NewSegment) {
        self.emit_row(&row, SegmentUpdateKind::Final);
        self.writes.push(row);
    }

    fn discard(&mut self, pending: &Pending) {
        log::debug!(target: "recording", "discarded a Me segment as echo ({} ms)", pending.row.start.0);
        if pending.draft_saved {
            self.removed.push(pending.row.id);
        }
        self.emit_row(&pending.row, SegmentUpdateKind::Discarded);
    }

    fn emit(&self, lane: Lane, row: &NewSegment, state: SegmentUpdateKind) {
        debug_assert_eq!(row.channel, lane.channel());
        self.emit_row(row, state);
    }

    fn emit_row(&self, row: &NewSegment, state: SegmentUpdateKind) {
        self.events.segment(&SegmentUpdate {
            meeting_id: row.meeting_id.clone(),
            segment_id: row.id,
            channel: row.channel,
            start: row.start,
            end: row.end,
            text: row.text.clone(),
            state,
        });
    }

    /// Releases expired holds and saves once per [`FLUSH`].
    fn tick(&mut self) {
        let now = self.host_clock.now();
        let verdicts = self.holdback.tick(now);
        self.apply(verdicts);
        if now.since(self.flushed_at) >= FLUSH {
            self.flushed_at = now;
            self.flush();
        }
    }

    /// Saves finals, discards and the drafts that may be saved now.
    fn flush(&mut self) {
        for lane in Lane::ALL {
            if lane == Lane::Me && !self.me_drafts_safe() {
                continue;
            }
            if let Some(draft) = self.drafts.unsaved(lane) {
                draft.dirty = false;
                draft.saved = true;
                let draft = draft.clone();
                self.writes.push(self.owner.draft_row(lane, &draft));
            }
        }
        if self.writes.is_empty() && self.removed.is_empty() {
            return;
        }
        match self.store.save(&self.writes, &self.removed) {
            Ok(()) => {
                self.writes.clear();
                self.removed.clear();
            }
            // Kept for the next flush (one transaction per batch).
            Err(error) => log::warn!(target: "recording", "{error}; retrying with the next batch"),
        }
    }

    /// Me drafts are saved only when they cannot be echo (headphones, or a quiet call with nothing held).
    fn me_drafts_safe(&self) -> bool {
        self.mode == EchoMode::Isolated || (!self.them_speaking && !self.holdback.is_holding())
    }

    fn finish(&mut self) {
        let verdicts = self.holdback.drain();
        self.apply(verdicts);
        self.flush();
        log::info!(target: "recording", "transcript router finished for meeting {}", self.owner.meeting);
    }
}
