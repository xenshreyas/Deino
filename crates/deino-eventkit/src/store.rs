//! The event store itself: authorization, the access prompt and creation.

#![allow(
    unsafe_code,
    reason = "objc2 marks every EventKit call unsafe; each takes valid, retained arguments"
)]

use std::sync::Mutex;

use block2::RcBlock;
use deino_core::calendar::CalendarAccess;
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_event_kit::{EKEntityType, EKEventStore};
use objc2_foundation::NSError;

use crate::convert;

/// Whether Deino may read events now. Never prompts; safe on any thread.
pub(crate) fn authorization() -> CalendarAccess {
    // SAFETY: a class method taking a valid entity type.
    let status = unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) };
    convert::access(status.0)
}

/// A new event store. Apple: create one and keep it, it is costly to make.
pub(crate) fn open() -> Retained<EKEventStore> {
    // SAFETY: plain object construction.
    unsafe { EKEventStore::new() }
}

/// Shows the macOS Calendars prompt if the user was never asked (macOS 14+
/// full access); `answered` receives whether access was granted. Returns at
/// once; the answer arrives on an `EventKit` queue.
pub(crate) fn request_full_access(
    store: &EKEventStore,
    answered: impl FnOnce(bool) + Send + 'static,
) {
    // The block is `Fn`, the answer comes once: hand the closure out once.
    let answered = Mutex::new(Some(answered));
    let block = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
        let pending = answered.lock().ok().and_then(|mut slot| slot.take());
        if let Some(answered) = pending {
            answered(granted.as_bool());
        }
    });
    // SAFETY: the block lives for the call and EventKit copies it before
    // returning; it captures only `Send` state.
    unsafe { store.requestFullAccessToEventsWithCompletion(RcBlock::as_ptr(&block)) };
}
