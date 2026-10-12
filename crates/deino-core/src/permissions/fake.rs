//! [`FakePermissionGate`]: an in-memory operating system for tests.

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use async_trait::async_trait;

use super::{PermissionGate, PermissionKind, PermissionState};

/// Permissions held in memory. Each kind starts
/// [`PermissionState::NotDetermined`]; a request grants it unless
/// [`FakePermissionGate::refuse`] was called. Settings pages open unless
/// [`FakePermissionGate::without_settings`] was used.
#[derive(Debug, Default)]
pub struct FakePermissionGate {
    states: Mutex<BTreeMap<PermissionKind, PermissionState>>,
    answers: Mutex<BTreeMap<PermissionKind, PermissionState>>,
    opened: Mutex<Vec<PermissionKind>>,
    no_settings: bool,
}

impl FakePermissionGate {
    /// A gate whose settings pages never open.
    pub fn without_settings() -> Self {
        Self {
            no_settings: true,
            ..Self::default()
        }
    }

    /// Sets what reading `kind` reports.
    pub fn set(&self, kind: PermissionKind, state: PermissionState) {
        lock(&self.states).insert(kind, state);
    }

    /// Makes requests for `kind` answer `answer` instead of granting it.
    pub fn refuse(&self, kind: PermissionKind, answer: PermissionState) {
        lock(&self.answers).insert(kind, answer);
    }

    /// The settings pages opened so far, in order.
    pub fn opened(&self) -> Vec<PermissionKind> {
        lock(&self.opened).clone()
    }
}

#[async_trait]
impl PermissionGate for FakePermissionGate {
    fn state(&self, kind: PermissionKind) -> PermissionState {
        lock(&self.states)
            .get(&kind)
            .copied()
            .unwrap_or(PermissionState::NotDetermined)
    }

    async fn request(&self, kind: PermissionKind) -> PermissionState {
        let answer = lock(&self.answers)
            .get(&kind)
            .copied()
            .unwrap_or(PermissionState::Granted);
        let mut states = lock(&self.states);
        if states.get(&kind) != Some(&PermissionState::Unknown) {
            states.insert(kind, answer);
        }
        answer
    }

    fn open_settings(&self, kind: PermissionKind) -> bool {
        lock(&self.opened).push(kind);
        !self.no_settings
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
