//! People and meeting attendance.
//!
//! [`PersonRepo`] stores the people the user meets, keyed for upserts by
//! [`EmailAddress`]; [`AttendeeRepo`] links them to meetings with a role and
//! an RSVP. [`SqlitePeopleRepo`] implements both and `FakePeopleRepo` stands
//! in for tests. ORG-04 adds the rules (upserts from calendar attendees and
//! speakers, merging, summaries) and the pages.

mod attendee_repo;
#[cfg(test)]
mod attendee_repo_tests;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod repo;
#[cfg(test)]
mod repo_tests;

pub use attendee_repo::AttendeeRepo;
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakePeopleRepo;
pub use model::{Attendee, EmailAddress, Person};
pub use repo::{PersonRepo, SqlitePeopleRepo};
