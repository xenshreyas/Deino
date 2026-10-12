//! Event linking (ORG-03): which calendar event a meeting belongs to, what
//! linking one brings into the meeting, and the meetings related to it.
//!
//! - **Rules** ([`nearest_event`], [`rank_suggestions`]): a meeting starting
//!   now belongs to the timed, not-declined event under way or starting
//!   within [`LINK_WINDOW`], the start closest to now first. The picker
//!   suggests the events of the [`SUGGEST_BEFORE`] before the meeting
//!   (Granola §3.1). Meeting detection (CAP-06) names its prompt with the
//!   same rule, so a detected call and its recording link the same event.
//! - **Linking** ([`EventLinkService`]): auto-link on start, link from the
//!   picker (also mid-recording) and unlink. Linking stores the link and the
//!   event's series on the meeting, takes the event's title, and turns its
//!   invitees into `meeting_attendees` rows through the calendar's
//!   [`AttendeeDirectory`](super::AttendeeDirectory) ([`EventAttendance`]);
//!   re-linking replaces the previous event's invitees. The description is
//!   read from the linked event by whoever needs it (NOT-02's prompt). A
//!   changed link makes the enhanced notes offer to regenerate
//!   (`RegenerateReason::EventChanged`).
//! - **Related meetings** ([`RelatedService`]): meetings of the same
//!   recurring series, or with exactly the same title and the same people
//!   (Granola §6.5), with the one just before this meeting as "Previous in
//!   series".
//! - **Last meeting** ([`LastMeetingService`], ORG-02): Coming up's "Review
//!   last meeting": the related meeting before an upcoming event, else the
//!   latest earlier one with exactly the same people.
//!
//! The link is `meetings.calendar_event_id` only (ADR 0012). Each write is
//! one statement on the writer: the link, then the attendees, then the
//! title; repeating a link converges, so a failure part-way is fixed by
//! linking again.

mod attendance;
mod error;
mod last_meeting;
mod model;
mod nearest;
#[cfg(test)]
mod nearest_tests;
mod related;
mod related_repo;
#[cfg(test)]
mod related_tests;
mod service;
#[cfg(test)]
mod service_tests;
#[cfg(test)]
mod test_people;

pub use attendance::{AttendeeChange, EventAttendance};
pub use error::LinkError;
pub use last_meeting::{LastMeetingService, PEOPLE_SCAN};
pub use model::{EventLinkChange, RelatedBy, RelatedMeeting, RelatedMeetings, TitleRule};
pub use nearest::{
    LINK_WINDOW, MAX_SUGGESTIONS, SUGGEST_AFTER, SUGGEST_BEFORE, is_linkable, link_range,
    nearest_event, rank_suggestions, suggestion_range,
};
pub use related::{RELATED_LIMIT, RelatedService};
pub use related_repo::{RelatedCandidate, RelatedMeetingRepo, RelationKey, SqliteRelatedRepo};
pub use service::{EventAgenda, EventLinkDeps, EventLinkService};
