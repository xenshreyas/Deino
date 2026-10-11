//! The meeting lifecycle as a state machine.
//!
//! A meeting's [`MeetingStatus`] only changes through a [`MeetingEvent`], and
//! [`MeetingStatus::apply`] is the one table of legal moves. Services compute
//! the next status here, then store it with a compare-and-set so two writers
//! cannot both move the same meeting.
//!
//! ```text
//! scheduled ─start_recording─▶ recording ─stop_recording─▶ processing ─complete─▶ ready
//!                                  │                           │                    │
//!                                  └────────fail───────┐  ┌────fail                 │
//!                                                      ▼  ▼                         │
//!                                                     failed                        │
//!                                                        │                          │
//!                         processing ◀───reprocess───────┴───────reprocess──────────┘
//!
//! A `ready` or `failed` meeting can also `resume_recording` (back to
//! `recording`): new audio is appended to it (CAP-07).
//! ```

db_enum! {
    /// Where a meeting is in its lifecycle. Stored in `meetings.status`.
    MeetingStatus {
        /// Created ahead of time from a calendar event; nothing recorded yet.
        Scheduled = "scheduled",
        /// Audio is being captured and transcribed.
        Recording = "recording",
        /// Recording stopped; the post-meeting pipeline is running.
        Processing = "processing",
        /// The pipeline finished; notes are available.
        Ready = "ready",
        /// The pipeline or the recording failed; the meeting can be reprocessed.
        Failed = "failed",
    }
}

db_enum! {
    /// Something that happens to a meeting and may change its status.
    MeetingEvent {
        /// Capture started for a scheduled meeting.
        StartRecording = "start_recording",
        /// Capture stopped; the pipeline takes over.
        StopRecording = "stop_recording",
        /// The pipeline finished every stage.
        Complete = "complete",
        /// Recording or processing failed.
        Fail = "fail",
        /// The user asked to run the pipeline again (re-transcribe, regenerate).
        Reprocess = "reprocess",
        /// Recording resumed on a finished meeting: new audio and transcript
        /// are appended (CAP-07).
        ResumeRecording = "resume_recording",
    }
}

/// A [`MeetingEvent`] that is not allowed in the meeting's current status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a {from} meeting cannot {event}")]
pub struct InvalidTransition {
    /// The status the meeting was in.
    pub from: MeetingStatus,
    /// The event that was rejected.
    pub event: MeetingEvent,
}

impl MeetingStatus {
    /// The status after `event`, or why the move is not allowed.
    pub fn apply(self, event: MeetingEvent) -> Result<Self, InvalidTransition> {
        use MeetingEvent as E;
        use MeetingStatus as S;
        match (self, event) {
            (S::Scheduled, E::StartRecording) | (S::Ready | S::Failed, E::ResumeRecording) => {
                Ok(S::Recording)
            }
            (S::Recording, E::StopRecording) | (S::Ready | S::Failed, E::Reprocess) => {
                Ok(S::Processing)
            }
            (S::Processing, E::Complete) => Ok(S::Ready),
            (S::Recording | S::Processing, E::Fail) => Ok(S::Failed),
            (from, event) => Err(InvalidTransition { from, event }),
        }
    }

    /// `scheduled` → `recording`.
    pub fn start_recording(self) -> Result<Self, InvalidTransition> {
        self.apply(MeetingEvent::StartRecording)
    }

    /// `recording` → `processing`.
    pub fn stop_recording(self) -> Result<Self, InvalidTransition> {
        self.apply(MeetingEvent::StopRecording)
    }

    /// `processing` → `ready`.
    pub fn complete(self) -> Result<Self, InvalidTransition> {
        self.apply(MeetingEvent::Complete)
    }

    /// `recording` or `processing` → `failed`.
    pub fn fail(self) -> Result<Self, InvalidTransition> {
        self.apply(MeetingEvent::Fail)
    }

    /// `ready` or `failed` → `processing`.
    pub fn reprocess(self) -> Result<Self, InvalidTransition> {
        self.apply(MeetingEvent::Reprocess)
    }

    /// `ready` or `failed` → `recording` (Resume on a finished meeting).
    pub fn resume_recording(self) -> Result<Self, InvalidTransition> {
        self.apply(MeetingEvent::ResumeRecording)
    }

    /// True while capture or the pipeline still owns the meeting.
    pub fn is_active(self) -> bool {
        matches!(self, Self::Recording | Self::Processing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_enum::assert_names_agree;

    #[test]
    fn a_meeting_walks_the_happy_path_from_scheduled_to_ready() {
        let ready = MeetingStatus::Scheduled
            .start_recording()
            .and_then(MeetingStatus::stop_recording)
            .and_then(MeetingStatus::complete);

        assert_eq!(ready, Ok(MeetingStatus::Ready));
    }

    #[test]
    fn failed_and_ready_meetings_can_be_reprocessed() {
        assert_eq!(
            MeetingStatus::Failed.reprocess(),
            Ok(MeetingStatus::Processing)
        );
        assert_eq!(
            MeetingStatus::Ready.reprocess(),
            Ok(MeetingStatus::Processing)
        );
    }

    #[test]
    fn exactly_the_documented_moves_are_legal() {
        use MeetingEvent as E;
        use MeetingStatus as S;

        let legal: Vec<(MeetingStatus, MeetingEvent)> = MeetingStatus::ALL
            .iter()
            .flat_map(|status| MeetingEvent::ALL.iter().map(|event| (*status, *event)))
            .filter(|(status, event)| status.apply(*event).is_ok())
            .collect();

        assert_eq!(
            legal,
            [
                (S::Scheduled, E::StartRecording),
                (S::Recording, E::StopRecording),
                (S::Recording, E::Fail),
                (S::Processing, E::Complete),
                (S::Processing, E::Fail),
                (S::Ready, E::Reprocess),
                (S::Ready, E::ResumeRecording),
                (S::Failed, E::Reprocess),
                (S::Failed, E::ResumeRecording),
            ]
        );
    }

    #[test]
    fn an_illegal_move_names_the_status_and_the_event() {
        let error = MeetingStatus::Ready.stop_recording().unwrap_err();

        assert_eq!(
            error,
            InvalidTransition {
                from: MeetingStatus::Ready,
                event: MeetingEvent::StopRecording
            }
        );
        assert_eq!(error.to_string(), "a ready meeting cannot stop_recording");
    }

    #[test]
    fn statuses_serialize_as_the_names_the_schema_checks() {
        assert_names_agree(MeetingStatus::ALL, MeetingStatus::as_str);
        assert_names_agree(MeetingEvent::ALL, MeetingEvent::as_str);
    }
}
