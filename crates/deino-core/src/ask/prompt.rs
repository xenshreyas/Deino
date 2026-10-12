//! [`AskContext`]: one meeting's material as the `chat` prompt shows it,
//! and the prompt for a question or a recipe after the thread so far.

use deino_llm::prompts::{AskSystem, AskUser, PromptText};
use deino_llm::{Message, Prompt};

use super::{AskMessage, AskRole};
use crate::enhance::{Material, MeetingInfo, SegmentIndex, TranscriptLine};

/// The most transcript characters one question reads (about 50k tokens);
/// a longer meeting is cut, saying so.
pub const MAX_TRANSCRIPT_CHARS: usize = 200_000;
/// The most earlier messages a prompt carries (three questions and answers).
pub const HISTORY_TURNS: usize = 6;
/// The answer cap.
const MAX_ANSWER_TOKENS: u32 = 2_048;
/// Where a long transcript is cut.
const CUT_NOTE: &str =
    "(The transcript is cut here: the meeting is longer than one question can read.)";

/// One meeting, rendered for the prompt, and its references.
#[derive(Clone, Debug)]
pub struct AskContext {
    meeting: String,
    notes: Option<String>,
    enhanced: Option<String>,
    transcript: String,
    index: SegmentIndex,
    empty: bool,
}

impl AskContext {
    /// The context of `material` (its transcript already redacted), with
    /// the user's notes and the enhanced notes as redacted Markdown.
    pub fn new(material: &Material, notes: Option<String>, enhanced: Option<String>) -> Self {
        let notes = notes.filter(|text| !text.trim().is_empty());
        let enhanced = enhanced.filter(|text| !text.trim().is_empty());
        Self {
            meeting: describe(&material.meeting),
            empty: material.transcript.is_empty() && notes.is_none() && enhanced.is_none(),
            transcript: transcript(&material.transcript),
            index: SegmentIndex::new(&material.transcript),
            notes,
            enhanced,
        }
    }

    /// True when there is nothing to answer from.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// The transcript's references, for resolving the answer's markers.
    pub fn index(&self) -> &SegmentIndex {
        &self.index
    }

    /// The prompt for `request` after `history` (oldest first).
    pub fn prompt(&self, history: &[AskMessage], request: &str) -> Prompt {
        let user = AskUser {
            meeting: &self.meeting,
            notes: self.notes.as_deref(),
            enhanced: self.enhanced.as_deref(),
            transcript: &self.transcript,
            request,
        };
        let mut messages = earlier(history);
        messages.push(Message::user(user.text()));
        Prompt {
            messages,
            ..Prompt::default()
        }
        .system(AskSystem.text())
        .max_tokens(MAX_ANSWER_TOKENS)
    }
}

/// What the model is asked: the question, or the recipe and what the user
/// added to it.
pub fn request_text(recipe_prompt: Option<&str>, question: &str) -> String {
    let question = question.trim();
    match recipe_prompt {
        None => question.to_owned(),
        Some(prompt) if question.is_empty() => {
            format!("Run this recipe on the meeting:\n\n<recipe>\n{prompt}\n</recipe>")
        }
        Some(prompt) => format!(
            "Run this recipe on the meeting:\n\n<recipe>\n{prompt}\n</recipe>\n\nThe user adds: {question}"
        ),
    }
}

/// The last [`HISTORY_TURNS`] messages as turns, starting with a question.
fn earlier(history: &[AskMessage]) -> Vec<Message> {
    let recent = history
        .get(history.len().saturating_sub(HISTORY_TURNS)..)
        .unwrap_or_default();
    recent
        .iter()
        .skip_while(|message| message.role == AskRole::Assistant)
        .map(|message| match message.role {
            AskRole::User => Message::user(question_label(message)),
            AskRole::Assistant => Message::assistant(message.plain_text()),
        })
        .collect()
}

/// A past question as the model saw it, in short.
fn question_label(message: &AskMessage) -> String {
    let text = message.plain_text();
    match &message.recipe {
        Some(recipe) if text.trim().is_empty() => format!("(Ran the recipe \"{}\".)", recipe.name),
        Some(recipe) => format!("(Ran the recipe \"{}\".) {text}", recipe.name),
        None => text,
    }
}

/// Title, date and attendees, one per line.
fn describe(meeting: &MeetingInfo) -> String {
    let mut lines = vec![
        format!("Title: {}", meeting.title),
        format!("Date: {}", meeting.date.format("%A %-d %B %Y, %H:%M UTC")),
    ];
    if !meeting.attendees.is_empty() {
        let names: Vec<String> = meeting
            .attendees
            .iter()
            .map(|a| {
                if a.is_me {
                    format!("{} (me)", a.name)
                } else {
                    a.name.clone()
                }
            })
            .collect();
        lines.push(format!("Attendees: {}", names.join(", ")));
    }
    lines.join("\n")
}

/// The referenced lines, cut at [`MAX_TRANSCRIPT_CHARS`].
fn transcript(lines: &[TranscriptLine]) -> String {
    if lines.is_empty() {
        return "The meeting has no transcript.".to_owned();
    }
    let mut text = String::new();
    for line in lines {
        let rendered = line.render();
        if text.len() + rendered.len() > MAX_TRANSCRIPT_CHARS {
            text.push_str(CUT_NOTE);
            break;
        }
        text.push_str(&rendered);
        text.push('\n');
    }
    text.trim_end().to_owned()
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
