//! Tags: coloured labels on meetings, applied by the user, the auto-tagger
//! or an automation.
//!
//! [`TagRepo`] stores the tags; [`MeetingTagRepo`] puts them on meetings
//! and records who did ([`TagSource`](deino_domain::TagSource)), so the
//! auto-tagger can leave meetings the user tagged alone. [`SqliteTagRepo`]
//! implements both and `FakeTagRepo` stands in for tests. ORG-06 adds the
//! rules and the auto-tag pipeline stage.

#[cfg(any(test, feature = "test-support"))]
mod fake;
mod meeting_tag_repo;
mod model;
mod repo;
#[cfg(test)]
mod tag_tests;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeTagRepo;
pub use meeting_tag_repo::MeetingTagRepo;
pub use model::{AppliedTag, Tag, TagApplication};
pub use repo::{SqliteTagRepo, TagRepo};
