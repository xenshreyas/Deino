//! Runs: an insight's latest answers on a meeting, and where a requested
//! run stands.

use chrono::{DateTime, Utc};
use deino_domain::{InsightId, InsightStatus, MeetingId};
use serde::Serialize;

use super::InsightAnswer;

/// One insight's latest run on one meeting.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InsightRun {
    /// The insight.
    pub insight_id: InsightId,
    /// The meeting.
    pub meeting_id: MeetingId,
    /// Pending, done or failed.
    pub status: InsightStatus,
    /// The answers of the last successful run (kept while a new one is
    /// pending or after one failed).
    pub answers: Vec<InsightAnswer>,
    /// The definition's revision the answers were written for; `None`
    /// before the first answer.
    pub revision: Option<u32>,
    /// Why the last run failed, user-safe.
    pub error: Option<String>,
    /// When the run was last asked for.
    pub requested_at: DateTime<Utc>,
    /// When the answers were written.
    pub generated_at: Option<DateTime<Utc>>,
}

impl InsightRun {
    /// True when the answers predate the definition's latest edit.
    pub fn is_stale(&self, current_revision: u32) -> bool {
        self.revision
            .is_some_and(|revision| revision < current_revision)
    }
}

/// A run with the meeting it is on, for the Insights page's tables.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingRun {
    /// The meeting's title.
    pub meeting_title: String,
    /// When the meeting started (or was saved).
    pub meeting_started_at: DateTime<Utc>,
    /// The run.
    pub run: InsightRun,
}

/// How many of an insight's runs are in each state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RunProgress {
    /// Waiting for the pipeline.
    pub pending: u32,
    /// Answered.
    pub done: u32,
    /// Failed.
    pub failed: u32,
}

/// A successful run, to store.
#[derive(Clone, Debug, PartialEq)]
pub struct RunAnswers {
    /// The insight.
    pub insight_id: InsightId,
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The definition's revision the answers answer.
    pub revision: u32,
    /// The validated, attributed answers.
    pub answers: Vec<InsightAnswer>,
    /// When they were written.
    pub at: DateTime<Utc>,
}
