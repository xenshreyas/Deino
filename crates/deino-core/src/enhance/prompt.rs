//! The `notes.enhance` prompts, rendered from the [`Material`] and the
//! template: one pass, a part of a long meeting (map), and the merge of the
//! parts' notes (reduce). Pure text building; the askama templates in
//! `deino-llm` place it.

use deino_llm::Prompt;
use deino_llm::prompts::{
    EnhanceMergeUser, EnhancePartSystem, EnhancePartUser, EnhanceSystem, EnhanceUser, PromptText,
};

use super::citations::SegmentIndex;
use super::material::{Material, MeetingInfo, PreviousNotes};
use super::transcript::{TranscriptLine, clock, render_lines};
use super::wire::{BlockKind, EnhancedDoc};
use crate::templates::{Template, template_prompt};

/// The most of a calendar description a prompt carries, in characters.
const MAX_DESCRIPTION_CHARS: usize = 2_000;
/// The most of the user's notes a prompt carries.
const MAX_NOTES_CHARS: usize = 12_000;
/// The most of the previous meeting's notes ("summary only, short").
const MAX_PREVIOUS_CHARS: usize = 1_200;
/// The most vocabulary terms a prompt lists.
const MAX_TERMS: usize = 100;
/// The answer cap of a whole meeting's notes (JSON is verbose).
const NOTES_MAX_TOKENS: u32 = 8_192;
/// The answer cap of one part's notes.
const PART_MAX_TOKENS: u32 = 4_096;

/// The rendered inputs every `notes.enhance` prompt shares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptInputs {
    meeting: String,
    template: String,
    previous: Option<String>,
    vocabulary: Option<String>,
    notes: Option<String>,
}

impl PromptInputs {
    /// The inputs of `material`, written with `template`.
    pub fn new(material: &Material, template: &Template) -> Self {
        let terms: Vec<&str> = material
            .vocabulary
            .iter()
            .take(MAX_TERMS)
            .map(String::as_str)
            .collect();
        Self {
            meeting: describe_meeting(&material.meeting),
            template: template_prompt(&template.content),
            previous: material.previous.as_ref().map(describe_previous),
            vocabulary: (!terms.is_empty()).then(|| terms.join(", ")),
            notes: material
                .notes
                .as_ref()
                .map(|notes| cut(notes.markdown.trim(), MAX_NOTES_CHARS)),
        }
    }

    /// One pass over the whole transcript.
    pub fn single(&self, lines: &[TranscriptLine]) -> Prompt {
        let transcript = transcript_or_none(lines);
        let user = EnhanceUser {
            meeting: &self.meeting,
            template: &self.template,
            previous: self.previous.as_deref(),
            vocabulary: self.vocabulary.as_deref(),
            notes: self.notes.as_deref(),
            transcript: &transcript,
        };
        finish(
            Prompt::new(user.text()).system(EnhanceSystem.text()),
            NOTES_MAX_TOKENS,
        )
    }

    /// Part `number` (from 1) of `count`: the map step.
    pub fn part(&self, number: usize, count: usize, lines: &[TranscriptLine]) -> Prompt {
        let user = EnhancePartUser {
            meeting: &self.meeting,
            template: &self.template,
            notes: self.notes.as_deref(),
            part: number,
            parts: count,
            span: &span_label(lines),
            transcript: &render_lines(lines),
        };
        finish(
            Prompt::new(user.text()).system(EnhancePartSystem.text()),
            PART_MAX_TOKENS,
        )
    }

    /// The merge of the parts' notes: the reduce step.
    pub fn merge(
        &self,
        parts: &[(&[TranscriptLine], EnhancedDoc)],
        index: &SegmentIndex,
    ) -> Prompt {
        let partials: Vec<String> = parts
            .iter()
            .enumerate()
            .map(|(at, (lines, notes))| render_part(at + 1, lines, notes, index))
            .collect();
        let user = EnhanceMergeUser {
            meeting: &self.meeting,
            template: &self.template,
            previous: self.previous.as_deref(),
            vocabulary: self.vocabulary.as_deref(),
            notes: self.notes.as_deref(),
            partials: &partials.join("\n\n"),
        };
        finish(
            Prompt::new(user.text()).system(EnhanceSystem.text()),
            NOTES_MAX_TOKENS,
        )
    }
}

/// The answer cap; the temperature stays the provider's default, which
/// every model accepts (some reasoning models refuse any other).
fn finish(prompt: Prompt, max_tokens: u32) -> Prompt {
    prompt.max_tokens(max_tokens)
}

fn transcript_or_none(lines: &[TranscriptLine]) -> String {
    if lines.is_empty() {
        "The meeting has no transcript: write the notes from the user's notes alone.".to_owned()
    } else {
        render_lines(lines)
    }
}

/// Title, date, calendar event, attendees and description, one per line.
fn describe_meeting(meeting: &MeetingInfo) -> String {
    let mut lines = vec![
        format!("Title: {}", meeting.title),
        format!("Date: {}", meeting.date.format("%A %-d %B %Y, %H:%M UTC")),
    ];
    if let Some(event) = meeting
        .calendar_title
        .as_deref()
        .filter(|t| *t != meeting.title)
    {
        lines.push(format!("Calendar event: {event}"));
    }
    if !meeting.attendees.is_empty() {
        let names: Vec<String> = meeting.attendees.iter().map(attendee_label).collect();
        lines.push(format!("Attendees ({}): {}", names.len(), names.join(", ")));
    }
    let description = meeting.description.trim();
    if !description.is_empty() {
        lines.push(format!(
            "Description: {}",
            cut(description, MAX_DESCRIPTION_CHARS)
        ));
    }
    lines.join("\n")
}

fn attendee_label(attendee: &super::material::AttendeeInfo) -> String {
    match (&attendee.title, attendee.is_me) {
        (_, true) => format!("{} (me)", attendee.name),
        (Some(title), false) => format!("{} ({title})", attendee.name),
        (None, false) => attendee.name.clone(),
    }
}

fn describe_previous(previous: &PreviousNotes) -> String {
    format!(
        "{}, on {}:\n{}",
        previous.title,
        previous.date.format("%-d %B %Y"),
        cut(previous.markdown.trim(), MAX_PREVIOUS_CHARS)
    )
}

/// One part's notes, every line ending with its valid references.
fn render_part(
    number: usize,
    lines: &[TranscriptLine],
    notes: &EnhancedDoc,
    index: &SegmentIndex,
) -> String {
    let mut out = vec![format!(
        "<part number=\"{number}\" span=\"{}\">",
        span_label(lines)
    )];
    for section in &notes.sections {
        out.push(format!("### {}", section.heading.trim()));
        for block in &section.blocks {
            out.push(format!(
                "{}{}",
                block_marker(block.kind),
                with_refs(&block.text, &block.evidence, index)
            ));
            for detail in &block.details {
                out.push(format!(
                    "  - {}",
                    with_refs(&detail.text, &detail.evidence, index)
                ));
            }
        }
    }
    out.push("</part>".to_owned());
    out.join("\n")
}

fn block_marker(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Todo => "- [ ] ",
        BlockKind::Paragraph => "",
        BlockKind::Bullet => "- ",
    }
}

fn with_refs(text: &str, references: &[String], index: &SegmentIndex) -> String {
    let valid: Vec<&str> = references
        .iter()
        .filter(|reference| index.contains(reference))
        .map(|reference| reference.trim())
        .collect();
    if valid.is_empty() {
        text.trim().to_owned()
    } else {
        format!("{} [{}]", text.trim(), valid.join(", "))
    }
}

/// `20:00–40:00`.
fn span_label(lines: &[TranscriptLine]) -> String {
    let start = lines.first().map(|line| line.start).unwrap_or_default();
    let end = lines.last().map(|line| line.end).unwrap_or_default();
    format!("{}–{}", clock(start), clock(end))
}

/// `text` cut to at most `limit` characters, at a line end when it can be.
fn cut(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let prefix: String = text.chars().take(limit).collect();
    let end = prefix.rfind('\n').unwrap_or(prefix.len());
    format!("{}\n…", prefix.get(..end).unwrap_or(&prefix).trim_end())
}

#[cfg(test)]
mod prompt_tests;
