//! How a template becomes prompt text: the per-section instructions and the
//! Markdown skeleton the summary fills ([`SummaryTemplate`]), and the
//! labelled blocks Settings → Templates previews ([`TemplatePreview`]).
//!
//! Pure functions, snapshot-tested: a wording change shows up in review.

use deino_domain::{Audience, Email, SectionFormat};
use serde::Serialize;

use super::{MeetingFacts, TemplateContent};

/// What the summary generator needs from a template
/// (`deino_core::summaries::SummaryRequest`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryTemplate {
    /// One instruction line per section, plus the title and meeting type.
    pub section_instructions: String,
    /// The skeleton to fill: a title and one heading per section.
    pub template_markdown: String,
}

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

/// How the LLM should write a section of each format.
pub fn format_directive(format: SectionFormat) -> &'static str {
    match format {
        SectionFormat::Bullets => "Write concise bullet points, one idea per bullet.",
        SectionFormat::Paragraph => "Write one or two short paragraphs of prose.",
        SectionFormat::Table => "Write a Markdown table with a header row and clear columns.",
        SectionFormat::Todo => {
            "Write a task list (`- [ ]`), one action per line, naming the owner and due date when known."
        }
    }
}

/// The instructions and skeleton the summary generator fills.
pub fn summary_template(content: &TemplateContent) -> SummaryTemplate {
    let mut lines = vec![
        "- **For the main title (`# [AI-Generated Title]`):** Analyze the entire transcript and create a concise, descriptive title for the meeting.".to_owned(),
        format!("- **Meeting type:** {}", meeting_type(content)),
    ];
    lines.extend(content.sections.iter().map(|section| {
        let instructions = join_sentences(&section.instructions, format_directive(section.format));
        format!("- **For the '{}' section:** {instructions}", section.title)
    }));
    let template_markdown = content.sections.iter().fold(
        String::from("# <Add Title here>\n\n"),
        |mut markdown, section| {
            markdown.push_str("## ");
            markdown.push_str(&section.title);
            markdown.push_str("\n\n");
            markdown
        },
    );
    SummaryTemplate {
        section_instructions: lines.join("\n"),
        template_markdown,
    }
}

/// The template rendered for `meeting`, after the user's profile context.
pub fn preview(
    content: &TemplateContent,
    meeting: &MeetingFacts,
    audience: Option<Audience>,
    profile_context: Option<String>,
) -> TemplatePreview {
    let summary = summary_template(content);
    let mut blocks = Vec::new();
    if let Some(context) = profile_context {
        blocks.push(block("About you", context));
    }
    blocks.push(block("This meeting", describe_meeting(meeting, audience)));
    blocks.push(block("Template", meeting_type(content)));
    blocks.push(block("Sections", summary.section_instructions));
    blocks.push(block(
        "Notes skeleton",
        summary.template_markdown.trim_end().to_owned(),
    ));
    TemplatePreview { blocks }
}

fn block(heading: &str, body: String) -> PromptBlock {
    PromptBlock {
        heading: heading.to_owned(),
        body,
    }
}

/// "Sales discovery. The user's team is selling…" (name, then context).
fn meeting_type(content: &TemplateContent) -> String {
    join_sentences(&format!("{}.", content.name), &content.meeting_context)
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
    fn the_summary_template_lists_every_section_with_its_format() {
        let summary = summary_template(&discovery());

        insta::assert_snapshot!("discovery_instructions", summary.section_instructions);
        insta::assert_snapshot!("discovery_skeleton", summary.template_markdown);
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
        assert_eq!(
            headings,
            [
                "About you",
                "This meeting",
                "Template",
                "Sections",
                "Notes skeleton"
            ]
        );
        insta::assert_snapshot!(preview.blocks[1].body);
    }
}
