//! How a template becomes prompt text: the template block every
//! `notes.enhance` prompt carries ([`template_prompt`], NOT-02), and the
//! labelled blocks Settings → Templates previews ([`TemplatePreview`]).
//!
//! Pure functions, snapshot-tested: a wording change shows up in review.

use deino_domain::{Audience, Email, SectionFormat};
use serde::Serialize;

use super::{MeetingFacts, TemplateContent};

/// One labelled part of a rendered template prompt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptBlock {
    /// What the block is, for example "This meeting".
    pub heading: String,
    /// The text the LLM receives.
    pub body: String,
}

/// A template rendered against a meeting, without calling an LLM.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePreview {
    /// The prompt's parts, in the order the LLM reads them.
    pub blocks: Vec<PromptBlock>,
}

/// How the LLM writes a section of each format, as note blocks.
pub fn section_directive(format: SectionFormat) -> &'static str {
    match format {
        SectionFormat::Bullets => "bullet blocks, one idea each",
        SectionFormat::Paragraph => "one or two paragraph blocks of prose",
        SectionFormat::Table => {
            "one bullet block per row, its columns as `column: value; column: value`"
        }
        SectionFormat::Todo => {
            "todo blocks, one action each, naming the owner and the due date when known"
        }
    }
}

/// The template as the notes prompt gives it: its name and meeting context,
/// then one numbered line per section with its instructions and format.
pub fn template_prompt(content: &TemplateContent) -> String {
    let mut lines = vec![
        join_sentences(&format!("{}.", content.name), &content.meeting_context),
        "Sections, in order:".to_owned(),
    ];
    for (number, section) in (1..).zip(&content.sections) {
        let write = format!("Write {}.", section_directive(section.format));
        let instructions = join_sentences(&section.instructions, &write);
        lines.push(format!("{number}. {}: {instructions}", section.title));
    }
    lines.join("\n")
}

/// The template rendered for `meeting`, after the user's profile context.
pub fn preview(
    content: &TemplateContent,
    meeting: &MeetingFacts,
    audience: Option<Audience>,
    profile_context: Option<String>,
) -> TemplatePreview {
    let mut blocks = Vec::new();
    if let Some(context) = profile_context {
        blocks.push(block("About you", context));
    }
    blocks.push(block("This meeting", describe_meeting(meeting, audience)));
    blocks.push(block("Template", template_prompt(content)));
    TemplatePreview { blocks }
}

fn block(heading: &str, body: String) -> PromptBlock {
    PromptBlock {
        heading: heading.to_owned(),
        body,
    }
}

/// Title, date, audience, attendees and description, one per line.
fn describe_meeting(meeting: &MeetingFacts, audience: Option<Audience>) -> String {
    let attendees: Vec<&str> = meeting.attendee_emails.iter().map(Email::as_str).collect();
    let mut lines = vec![format!("Title: {}", meeting.title)];
    if let Some(started_at) = meeting.started_at {
        lines.push(format!(
            "Date: {}",
            started_at.format("%A %-d %B %Y, %H:%M UTC")
        ));
    }
    if let Some(audience) = audience {
        lines.push(format!("Audience: {audience}"));
    }
    lines.push(format!(
        "Attendees ({}): {}",
        meeting.attendee_count,
        attendees.join(", ")
    ));
    if !meeting.description.trim().is_empty() {
        lines.push(format!("Description: {}", meeting.description.trim()));
    }
    lines.join("\n")
}

/// `first second`, or whichever is non-empty.
fn join_sentences(first: &str, second: &str) -> String {
    match (first.trim(), second.trim()) {
        ("", second) => second.to_owned(),
        (first, "") => first.to_owned(),
        (first, second) => format!("{first} {second}"),
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::{Email, MeetingId};
    use deino_testkit::at;

    use super::*;
    use crate::templates::CATALOG;

    fn discovery() -> TemplateContent {
        CATALOG
            .iter()
            .find(|template| template.id == "sales_discovery")
            .unwrap()
            .content()
    }

    fn meeting() -> MeetingFacts {
        MeetingFacts {
            id: MeetingId::new("meeting-1"),
            title: "Deino × Brightkite: discovery".into(),
            template_id: None,
            started_at: Some(at("2026-09-30T15:00:00Z")),
            description: "Intro call.".into(),
            attendee_emails: vec![Email::parse("marcus@brightkite-logistics.com").unwrap()],
            attendee_count: 2,
        }
    }

    #[test]
    fn the_template_prompt_numbers_every_section_with_its_format() {
        insta::assert_snapshot!("discovery_template_prompt", template_prompt(&discovery()));
    }

    #[test]
    fn the_preview_starts_with_the_profile_and_describes_the_meeting() {
        let preview = preview(
            &discovery(),
            &meeting(),
            Some(Audience::External),
            Some("About the user".into()),
        );

        let headings: Vec<&str> = preview
            .blocks
            .iter()
            .map(|block| block.heading.as_str())
            .collect();
        assert_eq!(headings, ["About you", "This meeting", "Template"]);
        insta::assert_snapshot!(preview.blocks[1].body);
    }
}
