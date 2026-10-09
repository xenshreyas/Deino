//! Deino's shared vocabulary, used by every other crate.
//!
//! This crate has no I/O and no Tauri. It holds the types that must mean the
//! same thing everywhere:
//!
//! - [`id`]: newtype identifiers ([`MeetingId`], UUIDv7 IDs), so IDs of
//!   different entities can never be mixed up;
//! - [`status`]: the meeting lifecycle state machine ([`MeetingStatus`]);
//! - [`enums`] and [`color`]: the other closed sets the core schema stores
//!   ([`Channel`], [`AttendeeRole`], [`Rsvp`], [`PaletteColor`], ...), each
//!   matching its column's `CHECK` constraint;
//! - [`units`]: [`Millis`] and [`Seconds`], so durations carry their unit;
//! - [`error`]: [`AppError`], the only error that crosses IPC, its
//!   [`ErrorCode`] and the [`ErrorClass`] trait that module errors implement;
//! - [`clock`]: the [`Clock`] port, so time-dependent logic is testable.
//!
//! Feature models and services live in `deino-core`; persistence lives in
//! `deino-db` (PARITY_PLAN §4.0).

#[macro_use]
mod db_enum;

pub mod clock;
pub mod color;
pub mod enums;
pub mod error;
pub mod id;
pub mod status;
pub mod units;

#[cfg(any(test, feature = "test-support"))]
pub use clock::FakeClock;
pub use clock::{Clock, SystemClock};
pub use color::{InvalidPaletteColor, PaletteColor};
pub use enums::{
    ActionItemOrigin, ActionItemPriority, ActionItemStatus, AttendeeRole, CalendarProvider,
    Channel, ConferencePlatform, MeetingSource, Rsvp, TagSource,
};
pub use error::{AppError, ErrorClass, ErrorCode};
#[cfg(debug_assertions)]
pub use id::ExampleNoteId;
pub use id::{
    ActionItemId, CalendarAccountId, CalendarEventId, CompanyId, FolderId, IdError, MeetingId,
    PersonId, SegmentId, SeriesId, SpeakerId, TagId, TemplateId,
};
pub use status::{InvalidTransition, MeetingEvent, MeetingStatus};
pub use units::{Millis, Seconds};
