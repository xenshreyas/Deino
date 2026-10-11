//! Custom AI insights (AI-06, ADR 0034): questions the user asks of their
//! meetings, answered with typed fields and attributed to the moment each
//! answer was said.
//!
//! An insight is a name, a description (the prompt), optional typed fields
//! (text, number, yes/no, date, one of a list), how many answers to keep
//! (Auto or N) and when it runs: after every meeting its conditions match
//! (`deino_domain::ConditionGroups`, shared with AUT-01), or only when asked.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs`, `input.rs` | [`InsightDef`], [`InsightInput`] and the rules a definition keeps |
//! | `gallery.rs`, `gallery/` | the twelve [`InsightTemplate`]s |
//! | `value.rs` | typed answers: [`InsightAnswer`], [`InsightValue`], [`FieldValue`] |
//! | `wire.rs`, `schema.rs`, `llm.rs` | the model's answer, the per-insight JSON schema, [`InsightModel`] on route `extract.insights` |
//! | `validate.rs` | wire values → typed values: repaired when close, dropped when not |
//! | `attribution.rs` | `s12` references → speaker, time and segment |
//! | `generate.rs` | [`GenerateService`], the `insights` pipeline stage's rules |
//! | `service.rs`, `views.rs`, `runs.rs`, `csv.rs` | [`InsightService`]: definitions, the tab and the page, runs on past meetings, CSV |
//! | `facts.rs` | what conditions read ([`MeetingRecords`], [`OwnDomains`]) |
//! | `repo.rs`, `results_repo.rs` | [`InsightRepo`], [`InsightResultRepo`], [`SpeakerNames`], all on [`SqliteInsightRepo`] |
//! | `run.rs` | [`InsightRun`]: an insight's latest answers on a meeting |
//! | `export.rs` | [`InsightExports`]: the Insights section of exports (EXP-01) |
//!
//! The model only reads and cites; Rust checks every value against its
//! field's type and resolves every citation against the transcript, so a
//! stored answer is always well-typed and attributed to a real moment.

mod attribution;
mod csv;
mod error;
mod export;
mod facts;
#[cfg(any(test, feature = "test-support"))]
mod fake;
#[cfg(any(test, feature = "test-support"))]
mod fake_ports;
mod gallery;
mod generate;
mod input;
mod llm;
mod model;
mod repo;
mod results_repo;
mod run;
mod runs;
mod schema;
mod service;
mod validate;
mod value;
mod views;
mod wire;

pub use attribution::Attribution;
pub use error::InsightError;
pub use export::InsightExports;
pub use facts::{MeetingRecord, MeetingRecords, OwnDomains, PastMeeting};
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeInsightStore;
#[cfg(any(test, feature = "test-support"))]
pub use fake_ports::{FixedDomains, RecordedScheduler, ScriptedInsightModel};
pub use gallery::{InsightTemplate, gallery, template};
pub use generate::{
    Attempt, GenerateDeps, GenerateOutcome, GenerateService, InsightGenerator, NO_TRANSCRIPT,
};
pub use input::{
    MAX_DESCRIPTION_CHARS, MAX_FIELD_NAME_CHARS, MAX_FIELDS, MAX_FIXED_ANSWERS, MAX_NAME_CHARS,
    MAX_OPTION_CHARS, MAX_OPTIONS,
};
pub use llm::{GatewayInsightModel, InsightModel, InsightRequest, NoInsightModel};
pub use model::{ApplyWhen, Cardinality, FieldKind, InsightDef, InsightField, InsightInput};
pub use repo::{InsightRepo, SqliteInsightRepo};
pub use results_repo::{InsightResultRepo, SpeakerNames};
pub use run::{InsightRun, MeetingRun, RunAnswers, RunFailure, RunProgress};
pub use runs::MAX_RUN_MEETINGS;
pub use schema::answer_schema;
pub use service::{InsightFiles, InsightPorts, InsightService, InsightSummary, RunScheduler};
pub use validate::{MAX_VALUE_CHARS, Validated, coerce, validate};
pub use value::{FieldEntry, FieldValue, InsightAnswer, InsightValue};
pub use views::{InsightTable, MeetingInsight, MeetingInsights, RunCandidate};
pub use wire::{WireAnswer, WireEvidence, WireInstance, WireScalar};

/// The most answers an Auto insight keeps per meeting.
pub const MAX_AUTO_ANSWERS: usize = 30;
