//! The lock's state machine on a fake clock and a fake authenticator.

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone, Utc};
use deino_domain::FakeClock;

use super::{AuthError, FakeAuthenticator, LockService, LockState};
use crate::settings::AppLockSettings;

const REASON: &str = "unlock Deino";

struct Harness {
    clock: Arc<FakeClock>,
    auth: Arc<FakeAuthenticator>,
    lock: LockService,
}

fn policy(enabled: bool) -> AppLockSettings {
    AppLockSettings {
        enabled,
        idle_minutes: 5,
        lock_on_wake: true,
    }
}

fn harness(enabled: bool) -> Harness {
    let clock = Arc::new(FakeClock::new(Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap()));
    let auth = Arc::new(FakeAuthenticator::default());
    let lock = LockService::new(clock.clone(), auth.clone(), policy(enabled));
    Harness { clock, auth, lock }
}

impl Harness {
    fn wait_minutes(&self, minutes: i64) {
        self.clock.advance(TimeDelta::minutes(minutes));
    }
}

#[test]
fn it_locks_after_the_idle_time() {
    let h = harness(true);

    h.wait_minutes(4);
    assert_eq!(h.lock.state(), LockState::Unlocked);
    h.wait_minutes(1);
    assert!(h.lock.is_locked());
}

#[test]
fn activity_restarts_the_idle_timer_but_never_unlocks() {
    let h = harness(true);

    h.wait_minutes(4);
    h.lock.activity();
    h.wait_minutes(4);
    assert_eq!(h.lock.state(), LockState::Unlocked);

    h.wait_minutes(5);
    assert_eq!(h.lock.activity(), LockState::Locked);
}

#[test]
fn a_disabled_lock_never_locks() {
    let h = harness(false);

    h.wait_minutes(600);
    h.lock.system_slept();

    assert_eq!(h.lock.lock_now(), LockState::Unlocked);
    assert!(!h.lock.is_locked());
}

#[test]
fn waking_locks_only_when_lock_on_wake_is_on() {
    let h = harness(true);
    h.lock.apply(AppLockSettings {
        lock_on_wake: false,
        ..policy(true)
    });
    assert_eq!(h.lock.system_slept(), LockState::Unlocked);

    h.lock.apply(policy(true));
    assert_eq!(h.lock.system_slept(), LockState::Locked);
}

#[test]
fn turning_the_lock_off_unlocks() {
    let h = harness(true);
    h.lock.lock_now();

    h.lock.apply(policy(false));

    assert_eq!(h.lock.state(), LockState::Unlocked);
}

#[tokio::test]
async fn unlocking_asks_the_owner_and_restarts_the_timer() {
    let h = harness(true);
    h.lock.lock_now();

    assert_eq!(h.lock.unlock(REASON).await, Ok(LockState::Unlocked));

    assert_eq!(h.auth.prompts(), 1);
    h.wait_minutes(4);
    assert!(!h.lock.is_locked());
}

#[tokio::test]
async fn a_cancelled_prompt_stays_locked() {
    let h = harness(true);
    h.lock.lock_now();
    h.auth.answer(Some(AuthError::Cancelled));

    assert_eq!(h.lock.unlock(REASON).await, Err(AuthError::Cancelled));

    assert!(h.lock.is_locked());
}

#[tokio::test]
async fn unlocking_an_unlocked_app_shows_no_prompt() {
    let h = harness(true);

    assert_eq!(h.lock.unlock(REASON).await, Ok(LockState::Unlocked));

    assert_eq!(h.auth.prompts(), 0);
}
