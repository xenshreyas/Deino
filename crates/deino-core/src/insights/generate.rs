//! [`GenerateService`]: the `insights` pipeline stage's rules.
//!
//! For one meeting:
//!
//! 1. Decide what to run. Runs the user asked for (pending, or failed
//!    earlier) when there are any; otherwise every automatic insight whose
//!    conditions match the meeting, which are marked pending first. Either
//!    way the work is a set of pending rows, so a retry resumes where the
//!    last attempt stopped and never pays for an answer twice.
//! 2. Read the meeting once: header, attendees, the numbered transcript.
//! 3. Per insight: ask the model ([`InsightModel`]), validate the answer
//!    against the fields ([`validate`]), attribute each answer to its line
//!    ([`Attribution`]), store it.
//!
//! A failure that may pass (a rate limit) stops the attempt so the pipeline
//! retries it, unless this is the last attempt; any other failure is stored
//! on the run, with a user-safe message, and the rest still run.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{Clock, InsightId, InsightStatus, LocalZone, MeetingId};
use deino_llm::extraction::{attendee_line, meeting_header};

use super::attribution::Attribution;
use super::validate::validate;
use super::{
    InsightDef, InsightError, InsightModel, InsightRepo, InsightRequest, InsightResultRepo,
    MeetingRecords, OwnDomains, RunAnswers,
};
use crate::enhance::{Material, MaterialSources, render_lines};

/// Stored on runs asked of a meeting that has no transcript.
pub const NO_TRANSCRIPT: &str = "The meeting has no transcript to read insights from.";

/// How a meeting's insights run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerateOutcome {
    /// No insight applies to the meeting, and none was asked for.
    NothingToRun,
    /// Insights were asked of a meeting without a transcript.
    NoTranscript,
    /// Every insight ran and was stored.
    Answered {
        /// How many insights ran.
        insights: usize,
    },
}

/// Whether the pipeline will try again after a failure that may pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attempt {
    /// More attempts follow: a rate limit stops this one.
    Retryable,
    /// The last attempt: every failure is stored on its run.
    Last,
}

/// Runs a meeting's insights: the port the `insights` stage calls.
#[async_trait]
pub trait InsightGenerator: Send + Sync {
    /// Runs what `meeting` needs (see the module docs).
    async fn generate(
        &self,
        meeting: &MeetingId,
        attempt: Attempt,
    ) -> Result<GenerateOutcome, InsightError>;
}

/// The ports generation works through.
#[derive(Clone)]
pub struct GenerateDeps {
    /// The definitions.
    pub defs: Arc<dyn InsightRepo>,
    /// The runs.
    pub results: Arc<dyn InsightResultRepo>,
    /// What conditions match on.
    pub records: Arc<dyn MeetingRecords>,
    /// Who counts as external.
    pub domains: Arc<dyn OwnDomains>,
    /// The meeting's transcript and people (NOT-02's sources).
    pub material: MaterialSources,
    /// The language model.
    pub model: Arc<dyn InsightModel>,
    /// Run times.
    pub clock: Arc<dyn Clock>,
    /// The user's time zone, for the prompt's date.
    pub zone: Arc<dyn LocalZone>,
}

/// Runs insights on meetings with the LLM.
pub struct GenerateService {
    deps: GenerateDeps,
}

impl GenerateService {
    /// The service over its ports.
    pub fn new(deps: GenerateDeps) -> Self {
        Self { deps }
    }

    /// The definitions to run on `meeting`, marked pending.
    async fn targets(&self, meeting: &MeetingId) -> Result<Vec<InsightDef>, InsightError> {
        let requested: Vec<InsightId> = self
            .deps
            .results
            .for_meeting(meeting)
            .await?
            .into_iter()
            .filter(|run| run.status != InsightStatus::Done)
            .map(|run| run.insight_id)
            .collect();
        let defs = self.deps.defs.list().await?;
        if !requested.is_empty() {
            return Ok(defs
                .into_iter()
                .filter(|def| requested.contains(&def.id))
                .collect());
        }
        let automatic = self.matching(meeting, defs).await?;
        let now = self.deps.clock.now();
        for def in &automatic {
            let meetings = std::slice::from_ref(meeting);
            self.deps.results.request(&def.id, meetings, now).await?;
        }
        Ok(automatic)
    }

    /// The automatic definitions whose conditions match `meeting`.
    async fn matching(
        &self,
        meeting: &MeetingId,
        defs: Vec<InsightDef>,
    ) -> Result<Vec<InsightDef>, InsightError> {
        let record = self
            .deps
            .records
            .record(meeting)
            .await?
            .ok_or_else(|| InsightError::MeetingNotFound(meeting.clone()))?;
        let facts = record.facts(&self.deps.domains.internal_domains().await);
        Ok(defs
            .into_iter()
            .filter(|def| {
                def.automatic_conditions()
                    .is_some_and(|conditions| conditions.matches(&facts))
            })
            .collect())
    }

    /// Runs one insight and stores its answers.
    async fn run(&self, def: &InsightDef, read: &MeetingRead) -> Result<(), InsightError> {
        let request = InsightRequest::new(
            def,
            read.header.clone(),
            read.attendees.clone(),
            read.transcript.clone(),
        );
        let wire = self.deps.model.answer(&request).await?;
        let answers = validate(&wire, &def.fields, def.cardinality)
            .into_iter()
            .map(|answer| read.attribution.attribute(answer))
            .collect();
        self.deps
            .results
            .save_answers(&RunAnswers {
                insight_id: def.id,
                meeting_id: read.meeting.clone(),
                revision: def.revision,
                answers,
                at: self.deps.clock.now(),
            })
            .await?;
        Ok(())
    }

    /// Stores `message` as the failure of each of `defs` on `meeting`.
    async fn fail_all(
        &self,
        meeting: &MeetingId,
        defs: &[InsightDef],
        message: &str,
    ) -> Result<(), InsightError> {
        for def in defs {
            let now = self.deps.clock.now();
            self.deps
                .results
                .save_failure(&def.id, meeting, message, now)
                .await?;
        }
        Ok(())
    }

    /// The meeting as every prompt shows it.
    fn read(&self, meeting: &MeetingId, material: &Material) -> MeetingRead {
        let info = &material.meeting;
        let attendees = info
            .attendees
            .iter()
            .map(|person| attendee_line(&person.name, person.is_me, person.title.as_deref()))
            .collect::<Vec<_>>()
            .join("\n");
        MeetingRead {
            meeting: meeting.clone(),
            header: meeting_header(&info.title, self.deps.zone.local(info.date)),
            attendees,
            transcript: render_lines(&material.transcript),
            attribution: Attribution::new(&material.transcript),
        }
    }
}

/// A meeting read once for all of its insights.
struct MeetingRead {
    meeting: MeetingId,
    header: String,
    attendees: String,
    transcript: String,
    attribution: Attribution,
}

#[async_trait]
impl InsightGenerator for GenerateService {
    async fn generate(
        &self,
        meeting: &MeetingId,
        attempt: Attempt,
    ) -> Result<GenerateOutcome, InsightError> {
        let targets = self.targets(meeting).await?;
        if targets.is_empty() {
            return Ok(GenerateOutcome::NothingToRun);
        }
        let material = self
            .deps
            .material
            .gather(meeting)
            .await?
            .ok_or_else(|| InsightError::MeetingNotFound(meeting.clone()))?;
        if material.transcript.is_empty() {
            self.fail_all(meeting, &targets, NO_TRANSCRIPT).await?;
            return Ok(GenerateOutcome::NoTranscript);
        }
        let read = self.read(meeting, &material);
        let mut first_failure = None;
        for def in &targets {
            let Err(error) = self.run(def, &read).await else {
                continue;
            };
            if error.is_retryable() && attempt == Attempt::Retryable {
                return Err(error);
            }
            self.fail_all(meeting, std::slice::from_ref(def), &error.to_string())
                .await?;
            first_failure.get_or_insert(error);
        }
        log::info!(target: "deino_core::insights", "ran {} insights on meeting {meeting}", targets.len());
        first_failure.map_or(
            Ok(GenerateOutcome::Answered {
                insights: targets.len(),
            }),
            Err,
        )
    }
}

#[cfg(test)]
#[path = "generate_tests.rs"]
mod tests;
