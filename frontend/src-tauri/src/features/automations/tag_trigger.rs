//! [`AutomationTagEvents`]: the tags' event port (`TagEvents`) that passes
//! each event on (to the UI) and hands newly applied tags to the
//! automations, AUT-01's tag trigger.
//!
//! Tags are built before automations (the automations' tag step applies
//! tags), so the service is found in Tauri state when a tag lands. The
//! event port is synchronous; the trigger's database work runs on the
//! async runtime and only stores run requests (plus scheduling a pipeline
//! stage), so it finishes quickly and is tracked by the pipeline after that.

use std::sync::Arc;

use deino_core::automations::AutomationService;
use deino_core::tags::{MeetingTagged, TagEvents};
use tauri::{AppHandle, Manager, Runtime};

/// Passes tag events on and triggers automations on them.
pub(crate) struct AutomationTagEvents<R: Runtime> {
    inner: Arc<dyn TagEvents>,
    app: AppHandle<R>,
}

impl<R: Runtime> AutomationTagEvents<R> {
    /// Wraps `inner`, finding the automations in `app`'s state.
    pub(crate) fn new(inner: Arc<dyn TagEvents>, app: AppHandle<R>) -> Self {
        Self { inner, app }
    }
}

impl<R: Runtime> TagEvents for AutomationTagEvents<R> {
    fn meeting_tagged(&self, event: &MeetingTagged) {
        self.inner.meeting_tagged(event);
        let Some(automations) = self.app.try_state::<Arc<AutomationService>>() else {
            return;
        };
        let automations = Arc::clone(&automations);
        let event = event.clone();
        tauri::async_runtime::spawn(async move {
            match automations
                .tag_applied(&event.meeting_id, &event.tag_id)
                .await
            {
                Ok(0) => {}
                Ok(runs) => log::info!(
                    target: "automations",
                    "tag {} on meeting {} started {runs} automation runs",
                    event.tag_id,
                    event.meeting_id
                ),
                Err(error) => log::warn!(
                    target: "automations",
                    "could not trigger automations for meeting {}: {error}",
                    event.meeting_id
                ),
            }
        });
    }
}
