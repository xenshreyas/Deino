//! The LLM fallback of template auto-selection: the [`TemplateClassifier`]
//! port and its gateway adapter, [`GatewayTemplateClassifier`] (route
//! `tagging`).
//!
//! It runs only when no auto rule matched and the user's default template is
//! "Auto", and only where an LLM call is acceptable (the post-meeting
//! pipeline), never when the UI merely shows which template applies.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{Email, TemplateId};
use deino_llm::prompts::{PromptText, TemplatePickSystem, TemplatePickUser};
use deino_llm::{Gateway, LlmError, Prompt, Route};
use schemars::JsonSchema;
use serde::Deserialize;

use super::{MeetingFacts, Template};

/// The longest explanation kept from the classifier, in characters.
const MAX_EXPLANATION_CHARS: usize = 120;
/// Room for an id and a short reason.
const PICK_MAX_TOKENS: u32 = 200;

/// The template the classifier chose and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Classification {
    /// One of the candidates.
    pub template_id: TemplateId,
    /// A short reason for the user.
    pub explanation: String,
}

/// Chooses a template for a meeting from the templates' descriptions.
#[async_trait]
pub trait TemplateClassifier: Send + Sync {
    /// One of `candidates` for `meeting`, or `None` when it cannot tell.
    async fn classify(
        &self,
        meeting: &MeetingFacts,
        candidates: &[Template],
    ) -> Result<Option<Classification>, LlmError>;
}

/// No LLM fallback: selection goes straight to the default template.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoClassifier;

#[async_trait]
impl TemplateClassifier for NoClassifier {
    async fn classify(
        &self,
        _meeting: &MeetingFacts,
        _candidates: &[Template],
    ) -> Result<Option<Classification>, LlmError> {
        Ok(None)
    }
}

/// [`TemplateClassifier`] over the LLM gateway's `tagging` route.
pub struct GatewayTemplateClassifier {
    gateway: Arc<Gateway>,
}

impl GatewayTemplateClassifier {
    /// A classifier over `gateway`.
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl TemplateClassifier for GatewayTemplateClassifier {
    async fn classify(
        &self,
        meeting: &MeetingFacts,
        candidates: &[Template],
    ) -> Result<Option<Classification>, LlmError> {
        let prompt = Prompt::new(pick_prompt(meeting, candidates))
            .system(TemplatePickSystem.text())
            .max_tokens(PICK_MAX_TOKENS);
        let answer: Pick = self.gateway.structured(Route::Tagging, prompt).await?;
        Ok(answer.accept(candidates))
    }
}

/// The classifier's structured answer.
#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct Pick {
    /// The chosen template's id, exactly as listed.
    template_id: String,
    /// Why, in under 15 words.
    reason: String,
}

impl Pick {
    /// The answer as a [`Classification`], if it names a candidate.
    pub(super) fn accept(self, candidates: &[Template]) -> Option<Classification> {
        let template = candidates
            .iter()
            .find(|candidate| candidate.id.as_str() == self.template_id.trim())?;
        let explanation: String = self
            .reason
            .trim()
            .chars()
            .take(MAX_EXPLANATION_CHARS)
            .collect();
        Some(Classification {
            template_id: template.id.clone(),
            explanation,
        })
    }
}

/// The user message: the meeting, then one line per candidate.
pub(super) fn pick_prompt(meeting: &MeetingFacts, candidates: &[Template]) -> String {
    let attendees: Vec<&str> = meeting.attendee_emails.iter().map(Email::as_str).collect();
    let mut lines = vec![format!("Title: {}", meeting.title)];
    if !meeting.description.trim().is_empty() {
        lines.push(format!("Description: {}", meeting.description.trim()));
    }
    lines.push(format!(
        "Attendees ({}): {}",
        meeting.attendee_count,
        attendees.join(", ")
    ));
    let templates: Vec<String> = candidates
        .iter()
        .map(|template| {
            let content = &template.content;
            format!(
                "- {}: {} — {}",
                template.id, content.name, content.description
            )
        })
        .collect();
    TemplatePickUser {
        meeting: &lines.join("\n"),
        templates: &templates.join("\n"),
    }
    .text()
}
