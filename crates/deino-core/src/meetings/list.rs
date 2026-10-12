//! The Meetings page's list (ORG-07): cursor pages of meetings outside the
//! trash, newest first, narrowed by a typed [`MeetingFilter`] (date range,
//! people, companies, tags, folders, action items, internal or external),
//! each row with its avatars, tags and length.
//!
//! [`ListQuery::resolve`] turns the user's filter into SQL parameters (local
//! days into UTC instants, the profile into the internal domains);
//! [`MeetingListRepo`] reads a page in three statements; and
//! [`MeetingListService`] pages and answers ⌘A ("every match").

mod filter;
mod model;
mod repo;
#[cfg(test)]
mod repo_tests;
mod service;
#[cfg(test)]
mod service_tests;

pub use filter::{AudienceQuery, DateRange, ListQuery, MeetingCursor, MeetingFilter};
pub use model::{
    AttendeeRow, ListedAttendee, ListedMeeting, ListedTag, MAX_PAGE_SIZE, MeetingPage, MeetingRow,
    TagRow,
};
pub use repo::{MeetingListRepo, RowLimit};
pub use service::{MeetingListDeps, MeetingListService, OwnAddresses};
