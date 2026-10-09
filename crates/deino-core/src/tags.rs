//! Tags: coloured labels on meetings, applied by the user, the auto-tagger
//! or an automation (ORG-06).
//!
//! - [`TagService`]: tag CRUD (name, [`PaletteColor`](deino_domain::PaletteColor),
//!   description, auto-apply switch) and applying tags to meetings with
//!   their [`TagSource`](deino_domain::TagSource), announced through
//!   [`TagEvents`] as [`MeetingTagged`] for automations (AUT-01).
//! - [`AutoTagService`]: the [`AutoTagger`] the `auto_tag` pipeline stage
//!   (order 70) calls. It asks the gateway's `tagging` route which existing
//!   tags fit, from tag descriptions plus up to three recent examples per
//!   tag ([`classifier`]), and skips meetings the user tagged by hand or
//!   when Settings turn it off ([`AutoTagSwitch`]).
//!
//! Ports: [`TagRepo`] (tags), [`MeetingTagRepo`] (who tagged which meeting)
//! and [`MeetingDigestReader`] (titles, summaries and transcripts for the
//! prompt). [`SqliteTagRepo`] implements all three; `FakeTagRepo` stands in
//! for tests.

mod auto_tag;
#[cfg(test)]
mod auto_tag_tests;
pub mod classifier;
mod digest_repo;
#[cfg(test)]
mod digest_repo_tests;
mod error;
mod events;
#[cfg(any(test, feature = "test-support"))]
mod fake;
#[cfg(any(test, feature = "test-support"))]
mod fake_digest;
mod input;
mod meeting_tag_repo;
mod model;
mod repo;
mod service;
#[cfg(test)]
mod service_tests;
#[cfg(test)]
mod tag_tests;

pub use auto_tag::{
    AutoTagPorts, AutoTagService, AutoTagSkip, AutoTagSwitch, AutoTagged, AutoTagger,
    NoModelAutoTagger,
};
pub use classifier::CONTENT_CHARS;
pub use digest_repo::{MeetingDigest, MeetingDigestReader};
pub use error::TagError;
#[cfg(any(test, feature = "test-support"))]
pub use events::RecordedTagEvents;
pub use events::{MeetingTagged, TagEvents};
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeTagRepo;
pub use input::{
    MAX_TAG_DESCRIPTION_CHARS, MAX_TAG_NAME_CHARS, NewTag, TagDescription, TagName, TagUpdate,
};
pub use meeting_tag_repo::MeetingTagRepo;
pub use model::{AppliedTag, MeetingTagLink, Tag, TagApplication};
pub use repo::{SqliteTagRepo, TagRepo};
pub use service::{TagPorts, TagService};
