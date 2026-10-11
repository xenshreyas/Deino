//! Sleep and wake, from `NSWorkspace` notifications.
//!
//! [`PowerWatch`] observes `NSWorkspaceWillSleepNotification` and
//! `NSWorkspaceDidWakeNotification` (anarlog's `detect/sleep.rs`, MIT,
//! reimplemented over objc2). The blocks run on the posting (main) thread
//! and only `try_send` an event; the observers are removed when the
//! subscription drops. Without a running main loop (tests, headless runs)
//! nothing is posted, and the supervisor's tick-gap check still notices a
//! sleep.

use std::ptr::NonNull;
use std::sync::mpsc::SyncSender;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_app_kit::{
    NSWorkspace, NSWorkspaceDidWakeNotification, NSWorkspaceWillSleepNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSNotificationName};

use crate::capture::events::{PowerEvent, Subscription, SystemEvent, SystemWatch};

/// Registered observers, removed on drop.
struct Observers {
    center: Retained<NSNotificationCenter>,
    tokens: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
}

// SAFETY: NSNotificationCenter is thread-safe, including `removeObserver:`
// from another thread; the tokens are only passed back to it.
#[allow(
    unsafe_code,
    reason = "NSNotificationCenter observers are removed from the supervisor's thread"
)]
unsafe impl Send for Observers {}

impl Subscription for Observers {}

impl Drop for Observers {
    #[allow(unsafe_code, reason = "removing observers this subscription added")]
    fn drop(&mut self) {
        for token in &self.tokens {
            // SAFETY: each token was returned by `addObserverForName:` on this center.
            unsafe { self.center.removeObserver(token.as_ref()) };
        }
    }
}

/// Watches the Mac's sleep and wake notifications.
#[derive(Clone, Copy, Debug, Default)]
pub struct PowerWatch;

impl SystemWatch for PowerWatch {
    fn watch(&self, events: SyncSender<SystemEvent>) -> Box<dyn Subscription> {
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        let tokens = notifications()
            .into_iter()
            .map(|(name, event)| observe(&center, name, events.clone(), event))
            .collect();
        Box::new(Observers { center, tokens })
    }
}

/// The notifications watched and the event each one means.
#[allow(unsafe_code, reason = "reads AppKit's notification-name statics")]
fn notifications() -> [(&'static NSNotificationName, PowerEvent); 2] {
    // SAFETY: AppKit's notification names are immutable statics.
    unsafe {
        [
            (NSWorkspaceWillSleepNotification, PowerEvent::WillSleep),
            (NSWorkspaceDidWakeNotification, PowerEvent::DidWake),
        ]
    }
}

#[allow(
    unsafe_code,
    reason = "registers a block observer with NSNotificationCenter"
)]
fn observe(
    center: &NSNotificationCenter,
    name: &NSNotificationName,
    events: SyncSender<SystemEvent>,
    event: PowerEvent,
) -> Retained<ProtocolObject<dyn NSObjectProtocol>> {
    let block = RcBlock::new(move |_note: NonNull<NSNotification>| {
        events.try_send(SystemEvent::Power(event)).ok();
    });
    // SAFETY: the block only sends on a thread-safe channel, so it may run on
    // any thread; the observer is removed before the block's captures drop.
    unsafe { center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block) }
}

/// Watches both device and power changes.
#[derive(Clone, Copy, Debug, Default)]
pub struct MacSystemWatch;

/// Both subscriptions, unregistered together when dropped.
struct Both {
    _devices: Box<dyn Subscription>,
    _power: Box<dyn Subscription>,
}

impl Subscription for Both {}

impl SystemWatch for MacSystemWatch {
    fn watch(&self, events: SyncSender<SystemEvent>) -> Box<dyn Subscription> {
        Box::new(Both {
            _devices: super::listener::CoreAudioWatch.watch(events.clone()),
            _power: PowerWatch.watch(events),
        })
    }
}

impl std::fmt::Debug for Observers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observers")
            .field("count", &self.tokens.len())
            .finish_non_exhaustive()
    }
}
