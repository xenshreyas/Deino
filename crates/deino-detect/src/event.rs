//! The typed stream detection publishes: a meeting was detected, it ended,
//! its prompt closed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::calendar::LinkedEvent;
use crate::model::{AppIdentity, MeetingKind, Platform};

/// Identifies one prompt, so a late click on a closed prompt does nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct DetectionId(pub u32);

/// A meeting the prompt offers to take notes for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    /// The prompt.
    pub id: DetectionId,
    /// The app the meeting is in ("Zoom", "Google Chrome").
    pub app: AppIdentity,
    /// The prompt's title: meeting, huddle or call.
    pub kind: MeetingKind,
    /// The service, when known.
    pub platform: Platform,
    /// The calendar event the call belongs to, if any.
    pub event: Option<LinkedEvent>,
    /// When the prompt appeared.
    pub detected_at: DateTime<Utc>,
    /// When recording starts on its own, if auto-start is on.
    pub auto_start_at: Option<DateTime<Utc>>,
}

/// One moment in the detection stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetectionEvent {
    /// Show the prompt for this meeting.
    MeetingDetected(Detection),
    /// The detected meeting ended: its app let go of the microphone.
    MeetingEnded(DetectionId),
    /// The prompt closed without the meeting ending: the user answered it,
    /// it timed out, recording started, or detection was switched off.
    PromptClosed(DetectionId),
}

impl DetectionEvent {
    /// The prompt this event is about.
    pub fn id(&self) -> DetectionId {
        match self {
            Self::MeetingDetected(detection) => detection.id,
            Self::MeetingEnded(id) | Self::PromptClosed(id) => *id,
        }
    }
}
