//! Auto-record rules (ORG-02): which calendar events record on their own.
//!
//! - **Policy** ([`decide`]): a pure function of the event's facts
//!   ([`EventFacts`]: internal or external, who organised it, the user's
//!   RSVP, its size), the global switches
//!   ([`AutoRecordSettings`](crate::settings::AutoRecordSettings)) and the
//!   user's [`Choices`]. [`starts_recording`] says which [`StartTrigger`]
//!   may start it: the meeting app taking the microphone (CAP-06), or the
//!   start time while the event's note is open.
//! - **Choices.** "Just this one" is `calendar_events.record_override`;
//!   "all occurrences" is a row in `calendar_series_choices`
//!   ([`SeriesChoiceRepo`]). The event's choice wins over the series'.
//! - **Service** ([`AutoRecordService`]): Coming up's [`PlannedEvent`]s and
//!   the record toggle ([`RecordChoice`]).
//!
//! The rules never start anything themselves: the app's reminder window,
//! Home's Join button and detection call the recorder.

mod choice_repo;
mod policy;
#[cfg(test)]
mod policy_tests;
mod service;
#[cfg(test)]
mod service_tests;

pub use choice_repo::SeriesChoiceRepo;
pub use policy::{
    Audience, Choices, EventFacts, MIN_MEETING_ATTENDEES, Organizer, RecordDecision, RecordReason,
    StartTrigger, decide, starts_recording,
};
pub use service::{AutoRecordDeps, AutoRecordService, ChoiceScope, PlannedEvent, RecordChoice};
