//! [`FollowUpInputs`]: what both follow-up calls read about a meeting, and
//! their prompts.
//!
//! The material comes from NOT-02's [`MaterialSources`], so the transcript
//! is redacted exactly as for notes when "Hide sensitive information" is on
//! (NOT-09); the notes (the enhanced notes, else the user's own) go through
//! the same redactor here. People are marked `(me)` or `(external)` by the
//! rule of [`super::judge`].

use deino_db::DbError;
use deino_domain::MeetingId;
use deino_llm::Prompt;
use deino_llm::prompts::{
    FOLLOW_UP_DECIDE_TASK, FOLLOW_UP_DRAFT_TASK, FollowUpDecideSystem, FollowUpDraftSystem,
    FollowUpUser, PromptText,
};

use super::Participant;
use crate::enhance::{Material, MaterialSources};
use crate::profile::InternalDomains;

/// The most characters of notes a prompt carries; longer notes are cut.
pub const MAX_NOTES_CHARS: usize = 12_000;
/// The most characters of transcript a prompt carries: its last lines,
/// where next steps and who sends what are usually agreed.
pub const MAX_TRANSCRIPT_CHARS: usize = 40_000;
/// What an empty section says.
const NONE: &str = "(none)";
/// The decision's answer budget.
const DECIDE_TOKENS: u32 = 512;
/// The draft's answer budget.
const DRAFT_TOKENS: u32 = 1_024;

/// The meeting as both prompts show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FollowUpInputs {
    /// `Title: …` and `Date: …`.
    pub meeting: String,
    /// One `- Name, Company (me|external)` line per person.
    pub people: String,
    /// The notes, redacted, or `(none)`.
    pub notes: String,
    /// `Speaker: text` lines, redacted, or `(none)`.
    pub transcript: String,
}

impl FollowUpInputs {
    /// The inputs of `meeting`, or `None` when it does not exist or there
    /// is nothing to read (no transcript and no notes).
    pub async fn gather(
        sources: &MaterialSources,
        meeting: &MeetingId,
        people: &[Participant],
        domains: &InternalDomains,
    ) -> Result<Option<Self>, DbError> {
        let Some(material) = sources.gather(meeting).await? else {
            return Ok(None);
        };
        let notes = notes(sources, &material).await?;
        if notes.is_none() && material.transcript.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self {
            meeting: header(&material),
            people: people_lines(people, domains),
            notes: notes.unwrap_or_else(|| NONE.to_owned()),
            transcript: transcript(&material),
        }))
    }

    /// The first call: is a follow-up warranted?
    pub fn decide_prompt(&self) -> Prompt {
        Prompt::new(self.user(FOLLOW_UP_DECIDE_TASK))
            .system(FollowUpDecideSystem.text())
            .max_tokens(DECIDE_TOKENS)
    }

    /// The second call: the draft.
    pub fn draft_prompt(&self) -> Prompt {
        Prompt::new(self.user(FOLLOW_UP_DRAFT_TASK))
            .system(FollowUpDraftSystem.text())
            .max_tokens(DRAFT_TOKENS)
    }

    fn user(&self, task: &str) -> String {
        FollowUpUser {
            meeting: &self.meeting,
            people: &self.people,
            notes: &self.notes,
            transcript: &self.transcript,
            task,
        }
        .text()
    }
}

/// The enhanced notes, else the user's own, redacted and cut to
/// [`MAX_NOTES_CHARS`]; `None` when there are neither.
async fn notes(sources: &MaterialSources, material: &Material) -> Result<Option<String>, DbError> {
    let enhanced = sources.history.latest(&material.meeting.id).await?;
    let markdown = enhanced
        .map(|notes| notes.markdown)
        .or_else(|| material.notes.as_ref().map(|notes| notes.markdown.clone()))
        .filter(|markdown| !markdown.trim().is_empty());
    let Some(markdown) = markdown else {
        return Ok(None);
    };
    let redacted = sources.redactor.redact(vec![markdown]).await;
    let text = redacted.into_iter().next().unwrap_or_default();
    Ok(Some(text.chars().take(MAX_NOTES_CHARS).collect()))
}

fn header(material: &Material) -> String {
    format!(
        "Title: {}\nDate: {}",
        material.meeting.title,
        material.meeting.date.format("%A %-d %B %Y, %H:%M UTC")
    )
}

/// One line per person: name, company, and `(me)` or `(external)`.
fn people_lines(people: &[Participant], domains: &InternalDomains) -> String {
    let lines: Vec<String> = people
        .iter()
        .map(|person| {
            let company = person
                .company
                .as_ref()
                .map(|company| format!(", {company}"))
                .unwrap_or_default();
            let mark = if person.is_me {
                " (me)"
            } else if person.is_external(domains) {
                " (external)"
            } else {
                ""
            };
            format!("- {}{company}{mark}", person.name)
        })
        .collect();
    lines.join("\n")
}

/// The transcript's last lines that fit [`MAX_TRANSCRIPT_CHARS`].
fn transcript(material: &Material) -> String {
    let mut kept: Vec<String> = Vec::new();
    let mut used = 0;
    for line in material.transcript.iter().rev() {
        let text = format!("{}: {}", line.speaker, line.text);
        used += text.chars().count() + 1;
        if used > MAX_TRANSCRIPT_CHARS {
            break;
        }
        kept.push(text);
    }
    if kept.is_empty() {
        return NONE.to_owned();
    }
    kept.reverse();
    kept.join("\n")
}
