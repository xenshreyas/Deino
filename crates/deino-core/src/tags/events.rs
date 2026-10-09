//! The tags event port: a tag landed on a meeting.
//!
//! The service never knows who listens. The app emits a typed Tauri event
//! (`meeting-tagged`) for automations (AUT-01) to trigger on; tests use
//! `RecordedTagEvents`. Cache freshness does not depend on it: `DbChanged`
//! already reports `meeting_tags`.

use deino_domain::{MeetingId, TagId, TagSource};

/// A tag was newly applied to a meeting (re-applying one it has is silent).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingTagged {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The tag.
    pub tag_id: TagId,
    /// Who applied it: the user, the auto-tagger or an automation.
    pub applied_by: TagSource,
}

/// Receives the tags feature's domain events.
pub trait TagEvents: Send + Sync {
    /// A tag was newly applied to a meeting.
    fn meeting_tagged(&self, event: &MeetingTagged);
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedTagEvents;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::Mutex;

    use super::{MeetingTagged, TagEvents};
    use crate::testing::lock;

    /// A [`TagEvents`] for tests: records every event in order.
    #[derive(Debug, Default)]
    pub struct RecordedTagEvents {
        tagged: Mutex<Vec<MeetingTagged>>,
    }

    impl RecordedTagEvents {
        /// Every `meeting_tagged` event so far, oldest first.
        pub fn tagged(&self) -> Vec<MeetingTagged> {
            lock(&self.tagged).clone()
        }
    }

    impl TagEvents for RecordedTagEvents {
        fn meeting_tagged(&self, event: &MeetingTagged) {
            lock(&self.tagged).push(event.clone());
        }
    }
}
