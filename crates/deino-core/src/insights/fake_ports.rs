//! In-memory adapters of the ports around insights, for tests: the user's
//! domains, the pipeline scheduler and the language model.

use std::collections::BTreeMap;
use std::sync::Mutex;

use async_trait::async_trait;
use deino_domain::{AppError, ErrorCode, MeetingId};
use deino_llm::LlmError;

use super::{InsightModel, InsightRequest, OwnDomains, RunScheduler, WireAnswer};
use crate::profile::InternalDomains;
use crate::testing::lock;

/// The user's own domains, fixed.
#[derive(Clone, Debug, Default)]
pub struct FixedDomains(pub InternalDomains);

#[async_trait]
impl OwnDomains for FixedDomains {
    async fn internal_domains(&self) -> InternalDomains {
        self.0.clone()
    }
}

/// A scheduler that records what it scheduled, failing for chosen meetings.
#[derive(Default)]
pub struct RecordedScheduler {
    scheduled: Mutex<Vec<MeetingId>>,
    refused: Mutex<Vec<MeetingId>>,
}

impl RecordedScheduler {
    /// Refuses to schedule `meeting`, as the pipeline does for a meeting
    /// that is still recording.
    pub fn refuse(&self, meeting: MeetingId) {
        lock(&self.refused).push(meeting);
    }

    /// The meetings scheduled, in order.
    pub fn scheduled(&self) -> Vec<MeetingId> {
        lock(&self.scheduled).clone()
    }
}

#[async_trait]
impl RunScheduler for RecordedScheduler {
    async fn schedule(&self, meeting: &MeetingId) -> Result<(), AppError> {
        if lock(&self.refused).contains(meeting) {
            return Err(AppError::new(
                ErrorCode::Invalid,
                "the meeting is still recording",
            ));
        }
        lock(&self.scheduled).push(meeting.clone());
        Ok(())
    }
}

/// A model that answers by insight name, and records what it was asked.
#[derive(Default)]
pub struct ScriptedInsightModel {
    answers: Mutex<BTreeMap<String, Vec<Result<WireAnswer, LlmError>>>>,
    asked: Mutex<Vec<InsightRequest>>,
}

impl ScriptedInsightModel {
    /// Answers the next request for the insight `name` with `answer`.
    #[must_use]
    pub fn answer(self, name: &str, answer: Result<WireAnswer, LlmError>) -> Self {
        lock(&self.answers)
            .entry(name.to_owned())
            .or_default()
            .push(answer);
        self
    }

    /// Every request, in order.
    pub fn asked(&self) -> Vec<InsightRequest> {
        lock(&self.asked).clone()
    }
}

#[async_trait]
impl InsightModel for ScriptedInsightModel {
    async fn answer(&self, request: &InsightRequest) -> Result<WireAnswer, LlmError> {
        lock(&self.asked).push(request.clone());
        let mut answers = lock(&self.answers);
        let queue = answers.entry(request.name.clone()).or_default();
        if queue.is_empty() {
            return Ok(WireAnswer::default());
        }
        queue.remove(0)
    }
}
