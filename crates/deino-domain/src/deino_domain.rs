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
//! - [`job`]: the post-meeting pipeline's [`JobStatus`] and [`RunOutcome`];
//! - [`units`]: [`Millis`] and [`Seconds`], so durations carry their unit;
//! - [`email`]: [`Email`], a normalised address (people, attendees, fixtures);
//! - [`bundle_id`]: [`BundleId`], a macOS app identifier (meeting detection);
//! - [`speaker_tag`]: [`CallApp`], [`DisplayName`] and [`TagSpan`], the names
//!   a meeting app's window shows for its active speaker (CAP-05);
//! - [`error`]: [`AppError`], the only error that crosses IPC, its
//!   [`ErrorCode`] and the [`ErrorClass`] trait that module errors implement;
//! - [`service`]: [`ServiceId`], every outside service Deino may talk to
//!   (FND-12), so settings, the egress log and `deino-net` name it alike;
//! - [`integration`]: the integrations framework's closed sets
//!   ([`IntegrationProvider`], [`Capability`], [`ConnectionStatus`], ...; INT-00);
//! - [`conditions`]: [`Condition`] and [`ConditionGroups`], which meetings
//!   an insight or an automation applies to (AI-06, AUT-01);
//! - [`clock`]: the [`Clock`] port, so time-dependent logic is testable.
//!
//! Feature models and services live in `deino-core`; persistence lives in
//! `deino-db` (PARITY_PLAN §4.0).

#[macro_use]
mod db_enum;

pub mod automation;
pub mod bundle_id;
pub mod clock;
pub mod color;
pub mod conditions;
pub mod email;
pub mod enums;
pub mod error;
pub mod id;
pub mod integration;
pub mod job;
pub mod service;
pub mod speaker_tag;
pub mod status;
pub mod units;
pub mod zone;

pub use automation::{AutomationRunStatus, AutomationStepStatus, AutomationTrigger};
pub use bundle_id::{BundleId, InvalidBundleId};
#[cfg(any(test, feature = "test-support"))]
pub use clock::FakeClock;
pub use clock::{Clock, SystemClock};
pub use color::{InvalidPaletteColor, PaletteColor};
pub use conditions::{Comparison, Condition, ConditionGroups, InvalidCondition, MeetingFacts};
pub use email::{Email, InvalidEmail};
pub use enums::{
    ActionItemOrigin, ActionItemPriority, ActionItemStatus, AttendeeRole, Audience,
    CalendarProvider, Channel, ConferencePlatform, FolderRuleKind, InsightApplyMode, InsightStatus,
    MeetingSource, Rsvp, SectionFormat, TagSource, VocabKind,
};
pub use error::{AppError, ErrorClass, ErrorCode};
#[cfg(debug_assertions)]
pub use id::ExampleNoteId;
pub use id::{
    ActionItemId, ActionItemViewId, AutomationId, AutomationRunId, CalendarAccountId,
    CalendarEventId, CalendarId, CompanyId, ConnectionId, DeliveryId, FolderId, FolderRuleId,
    IdError, InsightId, LlmCallId, MeetingId, MeetingViewId, PersonId, SegmentId, SeriesId,
    SpeakerId, StageId, TagId, TemplateId, VocabTermId, VoiceprintId,
};
pub use integration::{
    AuthMethod, Capability, ConnectionStatus, DeliveryStatus, IntegrationProvider,
};
pub use job::{JobStatus, RunOutcome};
pub use service::{ServiceId, ServiceKind};
pub use speaker_tag::{CallApp, DisplayName, InvalidDisplayName, MAX_DISPLAY_NAME_CHARS, TagSpan};
pub use status::{InvalidTransition, MeetingEvent, MeetingStatus};
pub use units::{Millis, Seconds};
pub use zone::{FixedZone, LocalZone, NOON, SystemZone};
