//! One worker per meeting: who owns a meeting, and its cancellation token.
//!
//! A meeting's stages must never run concurrently, so at most one worker
//! holds a meeting's slot. Waking a meeting that already has a worker asks
//! that worker to look at its jobs again instead of starting a second one;
//! a worker only gives the slot up when no such request is pending, so a
//! wake can never fall between "nothing left to do" and "slot released".

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use deino_domain::MeetingId;
use tokio_util::sync::CancellationToken;

/// The meetings that have a worker.
#[derive(Debug)]
pub(super) struct Slots {
    /// Cancelled on shutdown; every slot's token is its child.
    root: CancellationToken,
    slots: Mutex<HashMap<MeetingId, Slot>>,
}

#[derive(Debug)]
struct Slot {
    /// Cancels the meeting's current work.
    token: CancellationToken,
    /// Someone woke the meeting while its worker was busy.
    rescan: bool,
}

impl Slots {
    /// No slots yet; slot tokens are children of `root`.
    pub(super) fn new(root: CancellationToken) -> Self {
        Self {
            root,
            slots: Mutex::default(),
        }
    }

    fn slots(&self) -> MutexGuard<'_, HashMap<MeetingId, Slot>> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Claims `meeting` for a new worker and returns `true`. When a worker
    /// already holds it, asks that worker to rescan and returns `false`.
    pub(super) fn claim(&self, meeting: &MeetingId) -> bool {
        let mut slots = self.slots();
        if let Some(slot) = slots.get_mut(meeting) {
            slot.rescan = true;
            return false;
        }
        let token = self.root.child_token();
        slots.insert(meeting.clone(), Slot { token, rescan: false });
        true
    }

    /// The token that cancels the meeting's current work.
    pub(super) fn token(&self, meeting: &MeetingId) -> CancellationToken {
        self.slots()
            .get(meeting)
            .map_or_else(|| self.root.child_token(), |slot| slot.token.clone())
    }

    /// Cancels the meeting's current or next work. The token stays
    /// cancelled until the worker has acted on it and calls
    /// [`Slots::renew`], so a cancel between two steps is never lost.
    /// Returns `false` when no worker holds the meeting.
    pub(super) fn cancel(&self, meeting: &MeetingId) -> bool {
        let slots = self.slots();
        let Some(slot) = slots.get(meeting) else {
            return false;
        };
        slot.token.cancel();
        true
    }

    /// Arms a fresh token once the worker has acted on a cancel, so work
    /// scheduled afterwards (a re-run) is not cancelled too. Does nothing
    /// on shutdown.
    pub(super) fn renew(&self, meeting: &MeetingId) {
        if self.root.is_cancelled() {
            return;
        }
        if let Some(slot) = self.slots().get_mut(meeting)
            && slot.token.is_cancelled()
        {
            slot.token = self.root.child_token();
        }
    }

    /// Clears the meeting's rescan request and returns whether there was one.
    pub(super) fn take_rescan(&self, meeting: &MeetingId) -> bool {
        self.slots()
            .get_mut(meeting)
            .is_some_and(|slot| std::mem::take(&mut slot.rescan))
    }

    /// Gives the slot up and returns `true`, unless a rescan was requested
    /// meanwhile: then it clears the request, keeps the slot and returns `false`.
    pub(super) fn release(&self, meeting: &MeetingId) -> bool {
        let mut slots = self.slots();
        if let Some(slot) = slots.get_mut(meeting)
            && std::mem::take(&mut slot.rescan)
        {
            return false;
        }
        slots.remove(meeting);
        true
    }

    /// Gives the slot up whatever is pending: the worker is stopping.
    pub(super) fn abandon(&self, meeting: &MeetingId) {
        self.slots().remove(meeting);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meeting() -> MeetingId {
        MeetingId::new("meeting-a")
    }

    #[test]
    fn a_meeting_has_one_worker_and_a_second_wake_asks_it_to_rescan() {
        let slots = Slots::new(CancellationToken::new());

        assert!(slots.claim(&meeting()));
        assert!(!slots.claim(&meeting()));
        assert!(slots.take_rescan(&meeting()));
        assert!(!slots.take_rescan(&meeting()));
    }

    #[test]
    fn a_wake_before_release_keeps_the_worker_going() {
        let slots = Slots::new(CancellationToken::new());
        slots.claim(&meeting());
        slots.claim(&meeting());

        assert!(!slots.release(&meeting()), "the rescan request wins");
        assert!(slots.release(&meeting()));
        assert!(slots.claim(&meeting()), "released, so a new worker may claim it");
    }

    #[test]
    fn a_cancel_holds_until_the_worker_renews_the_token() {
        let slots = Slots::new(CancellationToken::new());
        assert!(!slots.cancel(&meeting()), "no worker, nothing to cancel");
        slots.claim(&meeting());
        let current = slots.token(&meeting());

        assert!(slots.cancel(&meeting()));
        assert!(current.is_cancelled());
        assert!(slots.token(&meeting()).is_cancelled(), "the next step sees it too");

        slots.renew(&meeting());
        assert!(!slots.token(&meeting()).is_cancelled());
    }

    #[test]
    fn shutting_down_cancels_every_slot() {
        let root = CancellationToken::new();
        let slots = Slots::new(root.clone());
        slots.claim(&meeting());

        root.cancel();

        assert!(slots.token(&meeting()).is_cancelled());
    }
}
