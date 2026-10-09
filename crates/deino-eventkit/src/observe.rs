//! Change notifications: the event store's (any calendar changed, access
//! granted or revoked) and the Mac waking from sleep. Each nudges the sync
//! scheduler, which coalesces bursts.

#![allow(
    unsafe_code,
    reason = "objc2 marks notification-centre registration unsafe; the blocks capture only Send state"
)]

use std::ptr::NonNull;

use block2::RcBlock;
use deino_core::calendar::{SyncTrigger, SyncTriggers};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidWakeNotification};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol, NSString};

/// The name `EventKit` posts when its database changes. The constant is not
/// in the objc2 bindings; Apple's notification names equal their symbols.
const STORE_CHANGED: &str = "EKEventStoreChangedNotification";

/// One registration, removed when dropped.
struct Registration {
    center: Retained<NSNotificationCenter>,
    token: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

impl Drop for Registration {
    fn drop(&mut self) {
        let observer: &AnyObject = (*self.token).as_ref();
        // SAFETY: the token came from this centre's own registration.
        unsafe { self.center.removeObserver(observer) };
    }
}

/// The store-changed and wake observers; dropping it unregisters both.
pub(crate) struct Observers {
    _registrations: [Registration; 2],
}

impl Observers {
    /// Nudges `triggers` on every store change and every wake.
    pub(crate) fn register(triggers: &SyncTriggers) -> Self {
        let store_changed = NSString::from_str(STORE_CHANGED);
        let local = NSNotificationCenter::defaultCenter();
        let workspace = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: an extern constant AppKit defines.
        let woke = unsafe { NSWorkspaceDidWakeNotification };
        Self {
            _registrations: [
                observe(
                    local,
                    &store_changed,
                    triggers.clone(),
                    SyncTrigger::SourceChanged,
                ),
                observe(workspace, woke, triggers.clone(), SyncTrigger::Woke),
            ],
        }
    }
}

/// Sends `trigger` whenever `center` posts `name`.
fn observe(
    center: Retained<NSNotificationCenter>,
    name: &NSString,
    triggers: SyncTriggers,
    trigger: SyncTrigger,
) -> Registration {
    let block = RcBlock::new(move |_: NonNull<NSNotification>| triggers.send(trigger));
    // SAFETY: no object filter and no queue (the block runs on the posting
    // thread and only does a non-blocking channel send); the centre copies
    // the block.
    let token = unsafe {
        center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block)
    };
    Registration { center, token }
}
