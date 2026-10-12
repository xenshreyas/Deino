//! The app lock off macOS: there is no system prompt to unlock with, and no
//! wake notification is observed.

use std::sync::Arc;

use deino_core::lock::{AuthError, LockService};

/// There is no system prompt to unlock with.
pub(super) const CAN_AUTHENTICATE: bool = false;

/// Nothing is observed.
pub(super) struct Observers;

/// This platform cannot authenticate the owner.
pub(super) fn authenticate(_reason: &str) -> Result<(), AuthError> {
    Err(AuthError::Unavailable)
}

/// Registers nothing.
pub(super) fn observe_wake(_lock: Arc<LockService>) -> Observers {
    Observers
}
