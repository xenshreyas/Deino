//! A meeting's thread: [`AskMessage`]s made of [`AnswerPart`]s, each a run
//! of text and the transcript segments it cites.

use chrono::{DateTime, Utc};
use deino_domain::{AskMessageId, MeetingId, Millis, RecipeId, SegmentId};
use serde::{Deserialize, Serialize};

/// Who wrote a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum AskRole {
    /// The user's question.
    User,
    /// The model's answer.
    Assistant,
}

/// A transcript moment an answer cites (NOT-03's excerpts open it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AnswerCitation {
    /// The segment.
    pub segment_id: SegmentId,
    /// Where it starts in the recording.
    pub start: Millis,
}

/// A run of text and the segments cited right after it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AnswerPart {
    /// The text, as the model wrote it, without its markers.
    pub text: String,
    /// What it cites, in the model's order, at most three.
    pub citations: Vec<AnswerCitation>,
}

impl AnswerPart {
    /// Text that cites nothing.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            citations: Vec::new(),
        }
    }
}

/// The recipe a question ran, as the thread labels it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeLabel {
    /// Its `/shortcut` when it ran.
    pub slug: String,
    /// Its name when it ran.
    pub name: String,
}

/// One message of a meeting's thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AskMessage {
    /// The message.
    pub id: AskMessageId,
    /// The meeting asked about.
    pub meeting_id: MeetingId,
    /// Who wrote it.
    pub role: AskRole,
    /// Its text runs; a question is one run without citations.
    pub parts: Vec<AnswerPart>,
    /// The recipe the question ran, if any.
    pub recipe: Option<RecipeLabel>,
    /// When it was written.
    pub created_at: DateTime<Utc>,
}

impl AskMessage {
    /// The text without citations, as later turns of the prompt show it.
    pub fn plain_text(&self) -> String {
        self.parts.iter().map(|part| part.text.as_str()).collect()
    }
}

/// What the user sends: a question, a recipe, or a recipe with words of
/// their own after it (`/write-follow-up-email keep it short`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AskRequest {
    /// What they typed (without the `/slug`); may be empty with a recipe.
    pub question: String,
    /// The recipe to run.
    pub recipe: Option<RecipeId>,
}
