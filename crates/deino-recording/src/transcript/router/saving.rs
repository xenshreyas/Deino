//! The router's saving side: batched writes, when Me drafts are safe to
//! save, and the final flush.

use super::State;
use crate::echo::EchoMode;
use crate::lane::Lane;

impl State {
    /// Saves finals, discards and the drafts that may be saved now.
    pub(super) fn flush(&mut self) {
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

    /// Judges everything still held and saves it all (the recording stops).
    pub(super) fn finish(&mut self) {
        let verdicts = self.holdback.drain();
        self.apply(verdicts);
        self.flush();
        log::info!(target: "recording", "transcript router finished for meeting {}", self.owner.meeting);
    }
}
