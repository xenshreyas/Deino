//! People and meeting attendance.
//!
//! [`PersonRepo`] stores the people the user meets, keyed for upserts by
//! [`Email`](deino_domain::Email); [`AttendeeRepo`] links them to meetings with a role and
//! an RSVP; [`MeRepo`] finds and moves the one person who is the user (kept
//! in step with the profile by `profile::ProfileService`). [`SqlitePeopleRepo`]
//! implements all three and `FakePeopleRepo` stands in for tests.
//! [`PeopleService::search`] finds people to @mention in the notepad
//! (NOT-01). ORG-04 adds the rules (upserts from calendar attendees and
//! speakers, merging, summaries) and the pages.

mod attendee_repo;
#[cfg(test)]
mod attendee_repo_tests;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod me_repo;
#[cfg(test)]
mod me_repo_tests;
mod model;
mod repo;
#[cfg(test)]
mod repo_tests;
mod service;

pub use attendee_repo::AttendeeRepo;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakePeopleRepo;
pub use me_repo::MeRepo;
pub use model::{Attendee, Person};
pub use repo::{PersonRepo, SqlitePeopleRepo};
pub use service::{MAX_SEARCH_CHARS, PeopleError, PeopleService, SEARCH_LIMIT};
