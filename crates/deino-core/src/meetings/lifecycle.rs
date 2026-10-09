//! A meeting's lifecycle and links: the columns FND-07 added to `meetings`.
//!
//! [`Meeting`](super::Meeting) stays the small list item the UI shows;
//! [`MeetingState`] is what the recorder (FND-05b), the pipeline (FND-08) and
//! the organisation features read and write through
//! [`MeetingLifecycleRepo`](super::MeetingLifecycleRepo).

use chrono::{DateTime, Utc};
use deino_domain::{
    CalendarEventId, MeetingId, MeetingSource, MeetingStatus, Seconds, SeriesId, TemplateId,
};

use super::MeetingTitle;

/// A meeting to create.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewMeeting {
    /// The new meeting's ID (`meeting-<uuid>`, unique).
    pub id: MeetingId,
    /// Its title.
    pub title: MeetingTitle,
    /// The status it starts in, usually `recording` or `scheduled`.
    pub status: MeetingStatus,
    /// How it came to exist.
    pub source: MeetingSource,
    /// When the row is created; also its first `updated_at`.
    pub created_at: DateTime<Utc>,
    /// When capture started, if it has.
    pub started_at: Option<DateTime<Utc>>,
    /// The folder holding the recording (`~/Movies/Deino/<meeting>`), if any.
    pub folder_path: Option<String>,
}

/// A meeting's lifecycle, timing, links and files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingState {
    /// The meeting's ID.
    pub id: MeetingId,
    /// Where the meeting is in its lifecycle.
    pub status: MeetingStatus,
    /// How the meeting came to exist.
    pub source: MeetingSource,
    /// When capture started; `None` for meetings saved before FND-07.
    pub started_at: Option<DateTime<Utc>>,
    /// When capture stopped.
    pub ended_at: Option<DateTime<Utc>>,
    /// The recorded length, excluding pauses.
    pub duration: Option<Seconds>,
    /// The calendar event the meeting was recorded for.
    pub calendar_event_id: Option<CalendarEventId>,
    /// The recurring series the meeting belongs to.
    pub series_id: Option<SeriesId>,
    /// The note template chosen for the meeting.
    pub template_id: Option<TemplateId>,
    /// When the meeting was moved to the trash.
    pub deleted_at: Option<DateTime<Utc>>,
    /// The meeting's recording files.
    pub audio: MeetingAudio,
}

/// When capture started and stopped, and how long it recorded.
///
/// `duration` is a denormalised copy of the audio tracks' length (see the
/// FND-07 migration); [`MeetingLifecycleRepo::set_timing`](super::MeetingLifecycleRepo::set_timing)
/// is the only writer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeetingTiming {
    /// When capture started.
    pub started_at: Option<DateTime<Utc>>,
    /// When capture stopped.
    pub ended_at: Option<DateTime<Utc>>,
    /// The recorded length, excluding pauses.
    pub duration: Option<Seconds>,
}

/// Absolute paths of a meeting's recording files; `None` when absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeetingAudio {
    /// Microphone and system audio mixed together.
    pub audio_path: Option<String>,
    /// The microphone track ("Me").
    pub audio_mic_path: Option<String>,
    /// The system audio track ("Them").
    pub audio_system_path: Option<String>,
    /// The screen recording (CAP-09).
    pub video_path: Option<String>,
}

/// What a meeting is linked to on the calendar.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeetingLink {
    /// The calendar event, or `None` to unlink.
    pub calendar_event_id: Option<CalendarEventId>,
    /// The event's recurring series, kept even when the event is gone.
    pub series_id: Option<SeriesId>,
}

/// A compare-and-set status change: applied only while the meeting is still
/// in `from`, so two writers cannot both move it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusChange {
    /// The status the caller saw.
    pub from: MeetingStatus,
    /// The status to move to.
    pub to: MeetingStatus,
    /// When the change happens; stamped on `updated_at`.
    pub at: DateTime<Utc>,
}
