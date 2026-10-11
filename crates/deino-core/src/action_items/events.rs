//! The [`ActionItemEvents`] port: action item changes, announced for the
//! features that react to them (two-way task sync in INT-03, automations in
//! AUT-01). The UI refreshes through `DbChanged` and does not need them.

use std::sync::Mutex;

use deino_domain::ActionItemId;
use serde::Serialize;

/// What happened to the items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ActionItemChangeKind {
    /// Added by the user or by extraction.
    Created,
    /// Fields changed (title, assignee, due day, ...).
    Updated,
    /// Marked done.
    Completed,
    /// Opened again.
    Reopened,
    /// Deleted, by the user or replaced by a regeneration.
    Deleted,
}

/// One change to one or more items.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemChange {
    /// What happened.
    pub kind: ActionItemChangeKind,
    /// The items it happened to.
    pub ids: Vec<ActionItemId>,
}

/// Where action item changes are announced.
pub trait ActionItemEvents: Send + Sync {
    /// The items in `change` changed; already saved.
    fn changed(&self, change: &ActionItemChange);
}

/// Announces nothing: for wiring without listeners.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoActionItemEvents;

impl ActionItemEvents for NoActionItemEvents {
    fn changed(&self, _change: &ActionItemChange) {}
}

/// Keeps every announced change, for tests.
#[derive(Debug, Default)]
pub struct RecordedActionItemEvents {
    changes: Mutex<Vec<ActionItemChange>>,
}

impl RecordedActionItemEvents {
    /// Every change so far, oldest first.
    pub fn changes(&self) -> Vec<ActionItemChange> {
        self.changes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ActionItemEvents for RecordedActionItemEvents {
    fn changed(&self, change: &ActionItemChange) {
        self.changes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(change.clone());
    }
}
