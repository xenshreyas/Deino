//! `LocalAuthentication` and the wake and screen-lock notifications, over
//! objc2.
//!
//! `LAContext` has no objc2 binding in the tree, so it is reached by
//! message sends on the class looked up at runtime (the framework is linked
//! below); a Mac without it reports [`AuthError::Unavailable`] instead of
//! crashing. The reply block runs on a private queue and only sends on a
//! channel; the caller waits on a blocking thread.

use std::ffi::CStr;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::mpsc::sync_channel;

use block2::RcBlock;
use deino_core::lock::{AuthError, LockService};
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, NSObjectProtocol, ProtocolObject};
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidWakeNotification};
use objc2_foundation::{
    NSDistributedNotificationCenter, NSError, NSNotification, NSNotificationCenter,
    NSNotificationName, NSString,
};

#[link(name = "LocalAuthentication", kind = "framework")]
extern "C" {}

/// `LocalAuthentication` can ask for Touch ID or the password.
pub(super) const CAN_AUTHENTICATE: bool = true;
/// The `LAContext` class.
const LA_CONTEXT: &CStr = c"LAContext";
/// `LAPolicyDeviceOwnerAuthentication`: Touch ID, Apple Watch or the
/// password.
const DEVICE_OWNER_AUTHENTICATION: isize = 2;
/// `LAError` codes the owner (or the system) cancelled with:
/// `userCancel`, `systemCancel`, `appCancel`.
const CANCEL_CODES: [isize; 3] = [-2, -4, -9];
/// `LAErrorPasscodeNotSet`: this Mac has no password to fall back to.
const PASSCODE_NOT_SET: isize = -5;
/// The distributed notification macOS posts when the screen locks.
const SCREEN_LOCKED: &str = "com.apple.screenIsLocked";

/// Shows the system prompt and waits for the owner's answer.
#[allow(
    unsafe_code,
    reason = "message sends to LAContext, which has no safe binding"
)]
pub(super) fn authenticate(reason: &str) -> Result<(), AuthError> {
    let class = AnyClass::get(LA_CONTEXT).ok_or(AuthError::Unavailable)?;
    // SAFETY: `+[LAContext new]` returns a new, retained context.
    let context: Option<Retained<AnyObject>> = unsafe { msg_send![class, new] };
    let context = context.ok_or(AuthError::Unavailable)?;
    let (answer, answered) = sync_channel(1);
    let reply = RcBlock::new(move |success: Bool, error: *mut NSError| {
        answer.try_send(outcome(success, error)).ok();
    });
    let reason = NSString::from_str(reason);
    // SAFETY: the selector and argument types match
    // `-[LAContext evaluatePolicy:localizedReason:reply:]`; the block only
    // sends on a channel, so it may run on any queue.
    unsafe {
        let () = msg_send![
            &*context,
            evaluatePolicy: DEVICE_OWNER_AUTHENTICATION,
            localizedReason: &*reason,
            reply: &*reply
        ];
    }
    // The context stays alive until the reply arrives.
    let result = answered.recv().unwrap_or(Err(AuthError::Failed));
    drop(context);
    result
}

/// The answer the reply block got.
#[allow(unsafe_code, reason = "reads the NSError the reply block was given")]
fn outcome(success: Bool, error: *mut NSError) -> Result<(), AuthError> {
    if success.as_bool() {
        return Ok(());
    }
    // SAFETY: LocalAuthentication passes a valid NSError (or nil) that lives
    // for the duration of the reply block.
    let code = unsafe { error.as_ref() }.map(NSError::code);
    match code {
        Some(code) if CANCEL_CODES.contains(&code) => Err(AuthError::Cancelled),
        Some(PASSCODE_NOT_SET) => Err(AuthError::Unavailable),
        _ => Err(AuthError::Failed),
    }
}

/// A notification center and the token of one observer registered on it.
type Observer = (
    Retained<NSNotificationCenter>,
    Retained<ProtocolObject<dyn NSObjectProtocol>>,
);

/// The registered observers, removed on drop.
pub(super) struct Observers {
    tokens: Vec<Observer>,
}

// SAFETY: NSNotificationCenter is thread-safe, including `removeObserver:`
// from another thread; the tokens are only passed back to it.
#[allow(
    unsafe_code,
    reason = "observers are held in Tauri state, which needs Send + Sync"
)]
unsafe impl Send for Observers {}
// SAFETY: as above; nothing reads the tokens.
#[allow(
    unsafe_code,
    reason = "observers are held in Tauri state, which needs Send + Sync"
)]
unsafe impl Sync for Observers {}

impl Drop for Observers {
    #[allow(unsafe_code, reason = "removing observers this value added")]
    fn drop(&mut self) {
        for (center, token) in &self.tokens {
            // SAFETY: each token was returned by `addObserverForName:` on its center.
            unsafe { center.removeObserver(token.as_ref()) };
        }
    }
}

/// Locks `lock` when the Mac wakes or its screen locks.
#[allow(unsafe_code, reason = "reads AppKit's notification-name static")]
pub(super) fn observe_wake(lock: Arc<LockService>) -> Observers {
    let workspace = NSWorkspace::sharedWorkspace().notificationCenter();
    let distributed: Retained<NSNotificationCenter> =
        Retained::into_super(NSDistributedNotificationCenter::defaultCenter());
    // SAFETY: AppKit's notification names are immutable statics.
    let woke = unsafe { NSWorkspaceDidWakeNotification };
    let screen_locked = NSString::from_str(SCREEN_LOCKED);
    let tokens = vec![
        (
            workspace.clone(),
            observe(&workspace, woke, Arc::clone(&lock)),
        ),
        (
            distributed.clone(),
            observe(&distributed, &screen_locked, lock),
        ),
    ];
    Observers { tokens }
}

#[allow(
    unsafe_code,
    reason = "registers a block observer with NSNotificationCenter"
)]
fn observe(
    center: &NSNotificationCenter,
    name: &NSNotificationName,
    lock: Arc<LockService>,
) -> Retained<ProtocolObject<dyn NSObjectProtocol>> {
    let block = RcBlock::new(move |_note: NonNull<NSNotification>| {
        let state = lock.system_slept();
        log::info!(target: "lock", "the Mac woke or locked its screen: {state:?}");
    });
    // SAFETY: the block only touches the lock service's mutex, so it may run
    // on any thread; the observer is removed before the block's captures drop.
    unsafe { center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block) }
}
