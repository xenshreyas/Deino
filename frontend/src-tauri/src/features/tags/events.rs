//! The `tags` plugin's typed event and the adapter that emits it.
//!
//! `TagService` announces each newly applied tag through its `TagEvents`
//! port; [`TauriTagEvents`] implements the port by emitting
//! [`MeetingTagged`] (`plugin:tags:meeting-tagged`).

use deino_core::tags::TagEvents;
use deino_domain::{MeetingId, TagId, TagSource};
use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// A tag was newly applied to a meeting, by the user, the auto-tagger or an
/// automation. Automations (AUT-01) trigger on it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct MeetingTagged {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The tag.
    pub tag_id: TagId,
    /// Who applied it.
    pub applied_by: TagSource,
}

impl From<&deino_core::tags::MeetingTagged> for MeetingTagged {
    fn from(event: &deino_core::tags::MeetingTagged) -> Self {
        Self {
            meeting_id: event.meeting_id.clone(),
            tag_id: event.tag_id,
            applied_by: event.applied_by,
        }
    }
}

/// Emits the tags feature's domain events to the UI.
pub struct TauriTagEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriTagEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> TagEvents for TauriTagEvents<R> {
    fn meeting_tagged(&self, event: &deino_core::tags::MeetingTagged) {
        // The tag is saved either way; a missed event only costs an
        // automation trigger, which AUT-01 reconciles from `meeting_tags`.
        if let Err(error) = MeetingTagged::from(event).emit(&self.app) {
            log::warn!(
                target: "tags",
                "could not emit meeting-tagged for meeting {}: {error}",
                event.meeting_id
            );
        }
    }
}
