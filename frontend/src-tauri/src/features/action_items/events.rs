//! The `action-items` plugin's typed event and the adapter that emits it.
//!
//! The core announces every change through its `ActionItemEvents` port;
//! [`TauriActionItemEvents`] implements the port by emitting
//! [`ActionItemChanged`] (`plugin:action-items:action-item-changed`), for the
//! listeners INT-03 (task sync) and AUT-01 (automations) add. The UI itself
//! refreshes through `DbChanged`.

use deino_core::action_items::{ActionItemChange, ActionItemChangeKind, ActionItemEvents};
use deino_domain::ActionItemId;
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// Action items were created, changed, completed, reopened or deleted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemChanged {
    /// What happened.
    pub kind: ActionItemChangeKind,
    /// The items it happened to.
    pub ids: Vec<ActionItemId>,
}

/// Emits action item changes to the webview.
pub struct TauriActionItemEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriActionItemEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> ActionItemEvents for TauriActionItemEvents<R> {
    fn changed(&self, change: &ActionItemChange) {
        let event = ActionItemChanged {
            kind: change.kind,
            ids: change.ids.clone(),
        };
        // The change is saved; a listener that misses the event reads it on
        // its next refresh.
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "action_items", "could not emit action-item-changed: {error}");
        }
    }
}
