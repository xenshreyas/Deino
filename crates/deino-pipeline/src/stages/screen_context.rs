//! `screen_context` (order 45): the text of the shared screen, for the
//! notes (CAP-09).
//!
//! Apple Vision reads each snapshot the capture kept, and Claude describes
//! it when the user turned that on (`deino_core::screen::ScreenContextService`).
//! It runs **before `enhance_notes` (50)**, not at the order 55 PARITY_PLAN
//! §4.2 first gave it, so NOT-02's notes are written with the slides' text
//! beside the transcript; no notes stage re-runs.
//!
//! - No new snapshots (none captured, or all read on an earlier run):
//!   skipped.
//! - No text recognition on this platform: skipped.
//! - A database failure: retried.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::screen::{ScreenContextError, ScreenContextReport, ScreenContextService};
use deino_domain::{MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "screen_context";
/// Where it runs: after the transcript's fixes, before the notes (50).
const ORDER: u16 = 45;
/// Shown when there was nothing new to read.
const NOTHING_TO_READ: &str = "The meeting has no new screen snapshots.";
/// Shown off macOS.
const NO_OCR: &str = "Reading the screen needs macOS.";

/// Reads the text of a meeting's snapshots (`ScreenContextService` in the app).
#[async_trait]
pub trait ScreenContextReader: Send + Sync {
    /// Reads and, when on, describes the snapshots of `meeting` not done yet.
    async fn read(&self, meeting: &MeetingId) -> Result<ScreenContextReport, ScreenContextError>;
}

#[async_trait]
impl ScreenContextReader for ScreenContextService {
    async fn read(&self, meeting: &MeetingId) -> Result<ScreenContextReport, ScreenContextError> {
        ScreenContextService::read(self, meeting).await
    }
}

/// Reads the shared screen's text before the notes are written.
pub struct ScreenContextStage {
    reader: Arc<dyn ScreenContextReader>,
}

impl ScreenContextStage {
    /// The stage over the screen context port.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            reader: Arc::clone(&ports.screen_context),
        }
    }
}

crate::registry::register_stage!(ScreenContextStage);

#[async_trait]
impl PipelineStage for ScreenContextStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        if ctx.cancel.is_cancelled() {
            return Err(StageError::Cancelled);
        }
        match self.reader.read(&meeting).await {
            Ok(report) if report == ScreenContextReport::default() => {
                Ok(StageOutcome::Skipped(NOTHING_TO_READ))
            }
            Ok(report) => {
                log::info!(target: "pipeline", "{ID}: read {} and described {} snapshots of {meeting}", report.read, report.described);
                Ok(StageOutcome::Done)
            }
            Err(ScreenContextError::Unsupported) => Ok(StageOutcome::Skipped(NO_OCR)),
            Err(error) => Err(StageError::retryable_from(&error)),
        }
    }
}

/// A reader that records the meetings it was asked about and answers with
/// a fixed report, for tests.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct RecordedScreenContext {
    asked: std::sync::Mutex<Vec<MeetingId>>,
    report: ScreenContextReport,
}

#[cfg(any(test, feature = "test-support"))]
impl RecordedScreenContext {
    /// A reader answering `report`.
    pub fn answering(report: ScreenContextReport) -> Self {
        Self {
            report,
            ..Self::default()
        }
    }

    /// The meetings asked about, in order.
    pub fn asked(&self) -> Vec<MeetingId> {
        self.asked
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[cfg(any(test, feature = "test-support"))]
#[async_trait]
impl ScreenContextReader for RecordedScreenContext {
    async fn read(&self, meeting: &MeetingId) -> Result<ScreenContextReport, ScreenContextError> {
        self.asked
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(meeting.clone());
        Ok(self.report)
    }
}

#[cfg(test)]
#[path = "screen_context_tests.rs"]
mod tests;
