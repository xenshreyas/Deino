//! [`LockService`]: the lock's state machine.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::Clock;
use serde::Serialize;

use super::{AuthError, Authenticator};
use crate::settings::AppLockSettings;

/// Whether the window may show the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum LockState {
    /// The library is visible.
    Unlocked,
    /// Only the lock screen (and recording) is available until the owner
    /// authenticates.
    Locked,
}

/// The lock's mutable state.
#[derive(Debug)]
struct Inner {
    policy: AppLockSettings,
    state: LockState,
    last_activity: DateTime<Utc>,
}

/// Locks after idle time and on wake; unlocks through the [`Authenticator`].
pub struct LockService {
    clock: Arc<dyn Clock>,
    authenticator: Arc<dyn Authenticator>,
    inner: Mutex<Inner>,
    /// One system prompt at a time.
    prompt: tokio::sync::Mutex<()>,
}

impl LockService {
    /// An unlocked service following `policy`; the idle timer starts now.
    pub fn new(
        clock: Arc<dyn Clock>,
        authenticator: Arc<dyn Authenticator>,
        policy: AppLockSettings,
    ) -> Self {
        let inner = Inner {
            policy,
            state: LockState::Unlocked,
            last_activity: clock.now(),
        };
        Self {
            clock,
            authenticator,
            inner: Mutex::new(inner),
            prompt: tokio::sync::Mutex::new(()),
        }
    }

    /// Follows changed settings. Turning the lock off unlocks; turning it
    /// on (or changing the idle time) restarts the idle timer.
    pub fn apply(&self, policy: AppLockSettings) {
        let now = self.clock.now();
        let mut inner = self.lock_inner();
        if !policy.enabled {
            inner.state = LockState::Unlocked;
        }
        inner.policy = policy;
        inner.last_activity = now;
    }

    /// The state now; an overdue idle lock takes effect here.
    pub fn state(&self) -> LockState {
        let now = self.clock.now();
        let mut inner = self.lock_inner();
        expire_idle(&mut inner, now);
        inner.state
    }

    /// True while locked: the IPC guard's question.
    pub fn is_locked(&self) -> bool {
        self.state() == LockState::Locked
    }

    /// The owner used the window: restarts the idle timer, unless the
    /// timer already ran out (activity never unlocks).
    pub fn activity(&self) -> LockState {
        let now = self.clock.now();
        let mut inner = self.lock_inner();
        expire_idle(&mut inner, now);
        if inner.state == LockState::Unlocked {
            inner.last_activity = now;
        }
        inner.state
    }

    /// Locks now, when the lock is on ("Lock Deino" in the menu).
    pub fn lock_now(&self) -> LockState {
        let mut inner = self.lock_inner();
        if inner.policy.enabled {
            inner.state = LockState::Locked;
        }
        inner.state
    }

    /// The Mac woke or its screen locked: locks when the settings say so.
    pub fn system_slept(&self) -> LockState {
        let mut inner = self.lock_inner();
        if inner.policy.enabled && inner.policy.lock_on_wake {
            inner.state = LockState::Locked;
        }
        inner.state
    }

    /// Asks the owner to authenticate with `reason`, and unlocks when they
    /// do. Without a lock in place no prompt is shown.
    pub async fn unlock(&self, reason: &str) -> Result<LockState, AuthError> {
        let _one_prompt = self.prompt.lock().await;
        if !self.is_locked() {
            return Ok(LockState::Unlocked);
        }
        self.authenticator.authenticate(reason).await?;
        let now = self.clock.now();
        let mut inner = self.lock_inner();
        inner.state = LockState::Unlocked;
        inner.last_activity = now;
        log::info!(target: "lock", "unlocked");
        Ok(LockState::Unlocked)
    }

    fn lock_inner(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding the lock leaves a whole `Inner` behind.
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl std::fmt::Debug for LockService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LockService")
            .field("inner", &*self.lock_inner())
            .finish_non_exhaustive()
    }
}

/// Locks `inner` when the lock is on and the idle time has run out.
fn expire_idle(inner: &mut Inner, now: DateTime<Utc>) {
    if !inner.policy.enabled || inner.state == LockState::Locked {
        return;
    }
    let idle = TimeDelta::minutes(i64::from(inner.policy.idle_minutes));
    if now - inner.last_activity >= idle {
        inner.state = LockState::Locked;
        log::info!(target: "lock", "locked after {} idle minutes", inner.policy.idle_minutes);
    }
}
