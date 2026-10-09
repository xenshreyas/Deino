//! Action items: commitments extracted from meetings, and standalone to-dos.
//!
//! [`ActionItemRepo`] stores them and keeps `completed_at` in step with the
//! status (the schema checks the pair). [`SqliteActionItemRepo`] implements
//! it and `FakeActionItemRepo` stands in for tests. AI-05 adds extraction,
//! the Action Items page and reminders; INT-03 adds the sync payload in
//! `external_refs_json`, which this module neither reads nor writes yet.

#[cfg(test)]
mod action_item_tests;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod repo;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeActionItemRepo;
pub use model::ActionItem;
pub use repo::{ActionItemRepo, SqliteActionItemRepo};
