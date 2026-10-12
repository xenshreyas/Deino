//! [`AskService`]: questions and recipes about one meeting, answered from
//! its redacted notes and transcript, with every citation resolved.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use deino_domain::{AskMessageId, Clock, MeetingId, RecipeId};
use deino_llm::TextSink;

use super::answer::parse_answer;
use super::prompt::request_text;
use super::{
    AnswerPart, AskContext, AskError, AskMessage, AskModel, AskRequest, AskRole, AskStore,
    RecipeLabel,
};
use crate::enhance::MaterialSources;
use crate::recipes::{Recipe, RecipeLookup};

/// What one question asks: the thread so far and the request.
#[derive(Clone, Copy)]
struct Turn<'a> {
    history: &'a [AskMessage],
    request: &'a str,
}

/// What the service is built from.
pub struct AskDeps {
    /// The meeting's material; its redactor also hides secrets in the notes.
    pub sources: MaterialSources,
    /// The threads.
    pub store: Arc<dyn AskStore>,
    /// The recipes a question may run.
    pub recipes: Arc<dyn RecipeLookup>,
    /// The model.
    pub model: Arc<dyn AskModel>,
    /// The time.
    pub clock: Arc<dyn Clock>,
}

/// "Ask about this meeting".
pub struct AskService {
    deps: AskDeps,
}

impl AskService {
    /// A service over `deps`.
    pub fn new(deps: AskDeps) -> Self {
        Self { deps }
    }

    /// The meeting's thread, oldest first.
    pub async fn thread(&self, meeting: &MeetingId) -> Result<Vec<AskMessage>, AskError> {
        Ok(self.deps.store.thread(meeting).await?)
    }

    /// Empties the meeting's thread.
    pub async fn clear(&self, meeting: &MeetingId) -> Result<(), AskError> {
        Ok(self.deps.store.clear(meeting).await?)
    }

    /// Answers `request` after the thread so far, streaming the answer to
    /// `sink`, and saves the question and the answer. Returns the answer.
    pub async fn ask(
        &self,
        meeting: &MeetingId,
        request: AskRequest,
        sink: &dyn TextSink,
    ) -> Result<AskMessage, AskError> {
        let question_text = request.question.trim().to_owned();
        let recipe = match request.recipe {
            Some(id) => Some(self.deps.recipes.runnable(&id).await?),
            None if question_text.is_empty() => return Err(AskError::EmptyQuestion),
            None => None,
        };
        let history = self.deps.store.thread(meeting).await?;
        let request = request_text(recipe.as_ref().map(|r| r.prompt.as_str()), &question_text);
        let asked_at = self.after(history.last().map(|message| message.created_at));
        let turn = Turn {
            history: &history,
            request: &request,
        };
        let parts = self.answer(meeting, turn, sink).await?;
        let question = AskMessage {
            recipe: recipe.as_ref().map(label),
            created_at: asked_at,
            ..self.message(meeting, AskRole::User, question_parts(question_text))
        };
        let answer = AskMessage {
            created_at: self.after(Some(asked_at)),
            ..self.message(meeting, AskRole::Assistant, parts)
        };
        self.deps.store.append(&question, &answer).await?;
        Ok(answer)
    }

    /// Runs `recipe` on `meeting` alone, without a thread, and saves
    /// nothing (the Recipes page's Preview).
    pub async fn preview(
        &self,
        meeting: &MeetingId,
        recipe: &RecipeId,
        sink: &dyn TextSink,
    ) -> Result<AskMessage, AskError> {
        let recipe = self.deps.recipes.runnable(recipe).await?;
        let request = request_text(Some(&recipe.prompt), "");
        let turn = Turn {
            history: &[],
            request: &request,
        };
        let parts = self.answer(meeting, turn, sink).await?;
        Ok(self.message(meeting, AskRole::Assistant, parts))
    }

    /// The model's answer to `turn`, its markers resolved.
    async fn answer(
        &self,
        meeting: &MeetingId,
        turn: Turn<'_>,
        sink: &dyn TextSink,
    ) -> Result<Vec<AnswerPart>, AskError> {
        let context = self.context(meeting).await?;
        let prompt = context.prompt(turn.history, turn.request);
        let text = self.deps.model.answer(prompt, sink).await?;
        log::debug!(target: "ask", "answered {meeting} in {} characters", text.len());
        Ok(parse_answer(&text, context.index()))
    }

    /// The meeting's material with its transcript and notes redacted.
    async fn context(&self, meeting: &MeetingId) -> Result<AskContext, AskError> {
        let sources = &self.deps.sources;
        let material = sources
            .gather(meeting)
            .await?
            .ok_or_else(|| AskError::MeetingNotFound(meeting.clone()))?;
        let enhanced = sources.history.latest(meeting).await?.map(|n| n.markdown);
        let notes = material.notes.as_ref().map(|n| n.markdown.clone());
        let mut redacted = sources
            .redactor
            .redact(vec![
                notes.unwrap_or_default(),
                enhanced.unwrap_or_default(),
            ])
            .await
            .into_iter();
        let context = AskContext::new(&material, redacted.next(), redacted.next());
        if context.is_empty() {
            return Err(AskError::NothingToRead);
        }
        Ok(context)
    }

    /// A new message, written now.
    fn message(&self, meeting: &MeetingId, role: AskRole, parts: Vec<AnswerPart>) -> AskMessage {
        AskMessage {
            id: AskMessageId::generate(self.deps.clock.as_ref()),
            meeting_id: meeting.clone(),
            role,
            parts,
            recipe: None,
            created_at: self.deps.clock.now(),
        }
    }

    /// Now, but always after `previous`, so a thread sorts in the order it
    /// was written whatever the clock does.
    fn after(&self, previous: Option<DateTime<Utc>>) -> DateTime<Utc> {
        let now = self.deps.clock.now();
        previous.map_or(now, |previous| {
            now.max(previous + Duration::milliseconds(1))
        })
    }
}

fn question_parts(text: String) -> Vec<AnswerPart> {
    if text.is_empty() {
        Vec::new()
    } else {
        vec![AnswerPart::plain(text)]
    }
}

fn label(recipe: &Recipe) -> RecipeLabel {
    RecipeLabel {
        slug: recipe.slug.clone(),
        name: recipe.name.clone(),
    }
}
