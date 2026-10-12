//! Deino's post-meeting pipeline (PARITY_PLAN §4.2, ADR 0018).
//!
//! When a meeting enters `processing`, the pipeline runs every registered
//! [`PipelineStage`] for it, in order: finalising audio, transcription
//! passes, notes, action items, indexing and so on. Work is durable: each
//! stage of each meeting is a row in `jobs`, each attempt a row in
//! `job_runs`, so a crash or a quit resumes where it stopped.
//!
//! - [`stage`]: the contract every stage implements ([`PipelineStage`],
//!   [`StageCtx`], [`StageOutcome`], [`StageError`]).
//! - [`registry`]: the one list of stages; adding a stage is one line there.
//! - [`stages`]: the stages themselves (`diarize`, `enhance_notes`, `auto_title`).
//! - [`PipelineService`]: the API the app calls (`enqueue`, `rerun`,
//!   `status`, `cancel`, `start`, `shutdown`), and [`PipelineTrigger`], which
//!   enqueues a meeting when the meetings lifecycle moves it to `processing`.
//! - The runner (private): one worker per meeting, so a meeting's stages run
//!   one at a time and in order; at most [`MAX_CONCURRENT_STAGES`] stages run
//!   across meetings; retryable failures back off by the injected `Clock`
//!   ([`RetryPolicy`]); every task is tracked, never detached.
//! - [`JobRepo`] and [`SqliteJobRepo`]: the queue in SQLite.
//! - [`ProgressSink`]: where [`PipelineProgress`] events go (a typed Tauri
//!   event in the app).
//!
//! This crate depends on `deino-core` for the meetings lifecycle and the
//! ports its stages read; the app wires adapters in `composition.rs`.

mod error;
mod events;
mod job;
mod plan;
mod reconcile;
pub mod registry;
mod repo;
#[cfg(test)]
mod repo_tests;
mod retry;
mod runner;
#[cfg(test)]
mod runner_tests;
mod service;
mod sleeper;
pub mod stage;
pub mod stages;
#[cfg(test)]
mod testing;
mod trigger;
#[cfg(test)]
mod voice_profile_tests;

pub use error::PipelineError;
#[cfg(any(test, feature = "test-support"))]
pub use events::RecordedProgress;
pub use events::{PipelineProgress, ProgressSink};
pub use job::{Job, JobKey, Settlement, StageStatus};
pub use registry::{Registry, StagePorts};
pub use repo::{JobRepo, SqliteJobRepo};
pub use retry::RetryPolicy;
pub use runner::MAX_CONCURRENT_STAGES;
pub use service::{PipelineDeps, PipelineService};
#[cfg(any(test, feature = "test-support"))]
pub use sleeper::FakeSleeper;
pub use sleeper::{ClockSleeper, Sleeper};
pub use stage::{PipelineStage, StageCtx, StageError, StageOutcome};
pub use stages::{
    AutoExportStage, AutoFolderStage, AutoTagStage, AutoTitleStage, AutomaticNotes,
    AutomationsStage, DiarizeStage, EnhanceNotesStage, FinalTranscriptionStage,
    IdentifySpeakersStage, IndexStage, MeetingTitler, NotesEnhancer, RepassSetting,
    ScreenContextReader, ScreenContextStage, UpdateEntitiesStage, VocabFixer, VocabFixupsStage,
};
#[cfg(any(test, feature = "test-support"))]
pub use stages::{
    FixedAutomaticNotes, FixedRepass, RecordedAutoFiling, RecordedAutoTags, RecordedAutomations,
    RecordedEnhancer, RecordedEntityUpdates, RecordedFixer, RecordedIdentifications,
    RecordedIndexing, RecordedScreenContext, RecordedTitler, ScriptedAutoExporter,
};
pub use trigger::PipelineTrigger;
