//! [`PermissionService`]: the permission rules over the [`PermissionGate`].

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;

use super::{PermissionError, PermissionKind, PermissionState, PermissionStatus};

/// The operating system's permissions (the port).
#[async_trait]
pub trait PermissionGate: Send + Sync {
    /// Where `kind` stands now, without prompting. [`PermissionState::Unknown`]
    /// when macOS can only tell by asking.
    fn state(&self, kind: PermissionKind) -> PermissionState;

    /// Asks for `kind`, showing the macOS prompt when it has not been
    /// answered, and reports the result.
    async fn request(&self, kind: PermissionKind) -> PermissionState;

    /// Opens the System Settings page where `kind` is granted; false when
    /// macOS would not open it.
    fn open_settings(&self, kind: PermissionKind) -> bool;
}

/// Reads, requests and opens the settings of every [`PermissionKind`].
pub struct PermissionService {
    gate: Arc<dyn PermissionGate>,
    /// The answer to the last request of a kind the gate cannot read.
    answered: Mutex<BTreeMap<PermissionKind, PermissionState>>,
}

impl PermissionService {
    /// A service over `gate`.
    pub fn new(gate: Arc<dyn PermissionGate>) -> Self {
        Self {
            gate,
            answered: Mutex::new(BTreeMap::new()),
        }
    }

    /// Every permission, in [`PermissionKind::ALL`] order.
    pub fn statuses(&self) -> Vec<PermissionStatus> {
        PermissionKind::ALL
            .into_iter()
            .map(|kind| PermissionStatus::new(kind, self.state(kind)))
            .collect()
    }

    /// Asks for `kind` and returns its status afterwards.
    pub async fn request(&self, kind: PermissionKind) -> PermissionStatus {
        let answer = self.gate.request(kind).await;
        self.remember(kind, answer);
        log::info!(target: "permissions", "requested {kind:?}: {answer:?}");
        PermissionStatus::new(kind, self.state(kind))
    }

    /// Opens the System Settings page for `kind`.
    pub fn open_settings(&self, kind: PermissionKind) -> Result<(), PermissionError> {
        if self.gate.open_settings(kind) {
            Ok(())
        } else {
            Err(PermissionError::SettingsUnavailable(kind))
        }
    }

    /// The live state, or the remembered answer when the gate cannot tell.
    fn state(&self, kind: PermissionKind) -> PermissionState {
        match self.gate.state(kind) {
            PermissionState::Unknown => self.answered(kind).unwrap_or(PermissionState::Unknown),
            live => live,
        }
    }

    fn answered(&self, kind: PermissionKind) -> Option<PermissionState> {
        let answered = self.answered.lock().unwrap_or_else(PoisonError::into_inner);
        answered.get(&kind).copied()
    }

    fn remember(&self, kind: PermissionKind, answer: PermissionState) {
        let mut answered = self.answered.lock().unwrap_or_else(PoisonError::into_inner);
        answered.insert(kind, answer);
    }
}
