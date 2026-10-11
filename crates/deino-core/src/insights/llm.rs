//! The [`InsightModel`] port and its gateway adapter: the one LLM call per
//! insight and meeting, on route `extract.insights` (PARITY_PLAN §4.3),
//! held to a schema built from the insight's own fields.

use std::sync::Arc;

use async_trait::async_trait;
use deino_llm::prompts::{InsightsSystem, InsightsUser, PromptText};
use deino_llm::{Gateway, JsonOutput, LlmError, Prompt, ProviderId, Route};

use super::schema::{answer_schema, count_line, field_lines};
use super::wire::WireAnswer;
use super::{Cardinality, InsightDef, InsightField};

/// Room for a long answer: an email, or thirty typed answers.
const INSIGHT_MAX_TOKENS: u32 = 4_096;
/// The schema's name, as providers that need one see it.
const SCHEMA_NAME: &str = "insight_answers";

/// One insight asked of one meeting, ready for the model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsightRequest {
    /// The title, then the local date with its weekday and time zone.
    pub meeting: String,
    /// One line per attendee.
    pub attendees: String,
    /// The numbered transcript (`[s12 04:31] Speaker: text`).
    pub transcript: String,
    /// The insight's name.
    pub name: String,
    /// Its description, the prompt.
    pub description: String,
    /// Its fields.
    pub fields: Vec<InsightField>,
    /// How many answers it keeps.
    pub cardinality: Cardinality,
}

impl InsightRequest {
    /// `def` asked of a meeting with this header, attendee list and transcript.
    pub fn new(def: &InsightDef, meeting: String, attendees: String, transcript: String) -> Self {
        Self {
            meeting,
            attendees,
            transcript,
            name: def.name.clone(),
            description: def.description.clone(),
            fields: def.fields.clone(),
            cardinality: def.cardinality,
        }
    }

    /// The gateway prompt: system instructions and the user message.
    pub fn prompt(&self) -> Prompt {
        let fields = field_lines(&self.fields);
        let user = InsightsUser {
            meeting: &self.meeting,
            attendees: &self.attendees,
            name: &self.name,
            description: &self.description,
            fields: fields.as_deref(),
            count: &count_line(self.cardinality),
            transcript: &self.transcript,
        };
        Prompt::new(user.text())
            .system(InsightsSystem.text())
            .max_tokens(INSIGHT_MAX_TOKENS)
    }

    /// The schema the answer is held to.
    pub fn output(&self) -> JsonOutput {
        JsonOutput {
            name: SCHEMA_NAME.to_owned(),
            schema: answer_schema(&self.fields, self.cardinality),
        }
    }
}

/// Answers an insight on a meeting.
#[async_trait]
pub trait InsightModel: Send + Sync {
    /// The model's answer to `request`, unvalidated.
    async fn answer(&self, request: &InsightRequest) -> Result<WireAnswer, LlmError>;
}

/// No model: for an app whose gateway could not be wired. Every run fails
/// as unavailable, and the stage waits for the user.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoInsightModel;

#[async_trait]
impl InsightModel for NoInsightModel {
    async fn answer(&self, _request: &InsightRequest) -> Result<WireAnswer, LlmError> {
        Err(LlmError::Unavailable {
            provider: ProviderId::BuiltIn,
            reason: "the AI gateway is not running".into(),
        })
    }
}

/// [`InsightModel`] over the LLM gateway.
pub struct GatewayInsightModel {
    gateway: Arc<Gateway>,
}

impl GatewayInsightModel {
    /// A model over `gateway`.
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl InsightModel for GatewayInsightModel {
    async fn answer(&self, request: &InsightRequest) -> Result<WireAnswer, LlmError> {
        self.gateway
            .structured_as(Route::ExtractInsights, request.prompt(), request.output())
            .await
    }
}
