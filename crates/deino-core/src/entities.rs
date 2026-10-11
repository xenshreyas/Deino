//! People and companies as entities (ORG-04, ADR 0036): the one place that
//! decides who becomes a person or a company, merges them, summarises them
//! and fetches company logos.
//!
//! `people` and `companies` own the rows and their CRUD ports; this module
//! owns the rules over them.
//!
//! | File | Role |
//! |---|---|
//! | `directory.rs` | [`EntityDirectory`]: the upsert rules (email, then name; companies from domains, never public or vanity ones), used by calendar sync, the pipeline and @mentions |
//! | `model.rs`, `fields.rs` | What the pages show ([`PersonPage`], [`CompanyPage`], list rows) and what the forms accept |
//! | `reads.rs`, `writes.rs` | [`EntityReads`] (lists with last-met dates and meeting counts computed on read, meetings, digests) and [`EntityWrites`] (single-field updates); [`SqliteEntityRepo`] implements both |
//! | `merge.rs`, `merge_repo.rs` | The pure merge rules and [`EntityMerger`], which moves every link in one transaction |
//! | `person_service.rs`, `company_service.rs`, `attendance.rs` | [`PersonService`], [`CompanyService`], [`AttendanceService`]: the pages' commands |
//! | `summary_prompt.rs`, `summarizer.rs` | The `entity.summary` prompt and [`EntitySummarizer`] |
//! | `updater.rs` | [`EntityUpdater`], the `update_entities` pipeline stage's work |
//! | `logos.rs`, `favicon.rs` | [`LogoService`] and its ports: the favicon over the `favicons` service, cached on disk |
//! | `upcoming.rs` | Calendar events coming up with someone |

mod attendance;
mod company_service;
mod directory;
mod error;
mod favicon;
mod fields;
mod logos;
mod merge;
mod merge_repo;
mod model;
mod names;
mod person_service;
mod ports;
mod reads;
mod summarizer;
mod summary_prompt;
mod upcoming;
mod updater;
mod writes;

#[cfg(test)]
mod fixture_entities_tests;
#[cfg(test)]
mod testing;

pub use attendance::{AttendanceService, MeetingPerson};
pub use company_service::{COMPANY_PAGE_MEETINGS, CompanyService};
pub use directory::EntityDirectory;
pub use error::EntityError;
pub use favicon::{FaviconSource, FsLogoFiles, MAX_LOGO_BYTES};
pub use logos::{ImageKind, Logo, LogoError, LogoFiles, LogoPorts, LogoService, LogoSource};
pub use merge::{CompanyMerge, EntityMerger, PersonMerge};
pub use model::{
    CompanyFields, CompanyPage, CompanyRow, EntityActionItem, EntityMeeting, PersonFields,
    PersonPage, PersonRow, PersonSeed, UpcomingEvent, Upserted,
};
pub use person_service::{PAGE_MEETINGS, PersonService};
pub use ports::EntityPorts;
pub use reads::{EntityDigest, EntityReads, SqliteEntityRepo};
pub use summarizer::{EntitySummarizer, SummaryOutcome};
pub use summary_prompt::{MEETING_CHARS, SUMMARY_MEETINGS};
pub use upcoming::{MAX_UPCOMING, UPCOMING_DAYS};
pub use updater::{
    EntityUpdate, EntityUpdatePorts, EntityUpdateService, EntityUpdater, MAX_SUMMARIES,
};
pub use writes::EntityWrites;

/// The longest person or company name accepted.
pub const MAX_NAME_CHARS: usize = 200;
