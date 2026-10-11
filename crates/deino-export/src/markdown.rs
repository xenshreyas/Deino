//! Markdown with YAML frontmatter: the export Obsidian and every notes app
//! open (EXP-01), and the files auto-export writes into a vault (EXP-02).
//!
//! The note starts with [`frontmatter`] properties, then the title, a meta
//! line, and one `##` section per chosen [`Section`]. Notes are re-written
//! from their [`Block`](crate::Block)s with their headings nested under the
//! section, and every piece of meeting text is escaped. In
//! [`LinkStyle::Obsidian`] people are `[[wiki links]]` (with [`person_note`]
//! pages to land on) and tags are `#tags`.

mod blocks;
mod frontmatter;
mod links;

pub use frontmatter::owner;
pub use links::{page_name, tag_slug};

use crate::labels::{self, Clock, heading};
use crate::{LinkStyle, MeetingExport, Participant, RenderOptions, Section, doc};

/// How far note headings move down so they nest under `##` sections.
const NOTE_DEMOTION: u8 = 2;

/// The meeting as a Markdown note.
pub fn render(export: &MeetingExport, options: &RenderOptions) -> String {
    note(export, options, None)
}

/// The meeting as a Markdown note; `transcript_link` names a transcript
/// sidecar note to link to instead of including the transcript.
pub fn note(
    export: &MeetingExport,
    options: &RenderOptions,
    transcript_link: Option<&str>,
) -> String {
    let mut parts = vec![
        frontmatter::meeting(export, options.links).build(),
        format!("# {}", blocks::escape(&export.title)),
        meta_line(export, options.links),
    ];
    for section in export.sections(options.sections) {
        parts.push(section_body(export, section, options.links));
    }
    if let Some(link) = transcript_link {
        parts.push(format!("## {}\n\n[[{link}]]", heading::TRANSCRIPT));
    }
    finish(&parts)
}

/// The transcript alone, as the sidecar note auto-export writes beside a
/// meeting note.
pub fn transcript_note(export: &MeetingExport, options: &RenderOptions) -> String {
    let title = format!("{} – {}", export.title, heading::TRANSCRIPT);
    let properties = frontmatter::meeting(export, options.links).text("part", "transcript");
    finish(&[
        properties.build(),
        format!("# {}", blocks::escape(&title)),
        transcript(export),
    ])
}

/// A person's page in an Obsidian vault, so their `[[wiki link]]` lands
/// somewhere. Written once; never overwritten.
pub fn person_note(person: &Participant) -> String {
    let mut properties = frontmatter::Frontmatter::default().list("tags", &["person".to_owned()]);
    if let Some(email) = &person.email {
        properties = properties.text("email", email);
    }
    finish(&[
        properties.build(),
        format!("# {}", blocks::escape(&person.name)),
    ])
}

fn finish(parts: &[String]) -> String {
    let mut note = parts.join("\n\n");
    note.push('\n');
    note
}

/// "Thursday, October 8, 2026 at 15:31 · 7 min", then the tags in Obsidian mode.
fn meta_line(export: &MeetingExport, style: LinkStyle) -> String {
    let mut meta = labels::long_date(&export.started_at);
    if let Some(duration) = export.duration {
        meta = format!("{meta} · {}", labels::duration(duration));
    }
    match style {
        LinkStyle::Obsidian if !export.tags.is_empty() => {
            format!("{meta}\\\n{}", links::tag_line(&export.tags))
        }
        _ => meta,
    }
}

fn section_body(export: &MeetingExport, section: Section, style: LinkStyle) -> String {
    let (title, body) = match section {
        Section::Attendees => (heading::ATTENDEES, attendees(export, style)),
        Section::Notes => (heading::NOTES, notes(export.notes.as_deref())),
        Section::MyNotes => (heading::MY_NOTES, notes(export.my_notes.as_deref())),
        Section::ActionItems => (heading::ACTION_ITEMS, action_items(export, style)),
        Section::Insights => (heading::INSIGHTS, insights(export)),
        Section::Transcript => (heading::TRANSCRIPT, transcript(export)),
    };
    format!("## {title}\n\n{body}")
}

/// Each insight as a `###` heading over its answers, one list item each,
/// then who said it and when.
fn insights(export: &MeetingExport) -> String {
    export
        .insights
        .iter()
        .map(|insight| {
            let answers: Vec<String> = insight
                .answers
                .iter()
                .map(|answer| {
                    let line = blocks::escape(&answer.line()).replace('\n', "\n  ");
                    match answer.attribution() {
                        Some(source) => format!("- {line} — {}", blocks::escape(&source)),
                        None => format!("- {line}"),
                    }
                })
                .collect();
            let name = blocks::escape(&insight.name);
            format!("### {name}\n\n{}", answers.join("\n"))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn attendees(export: &MeetingExport, style: LinkStyle) -> String {
    export
        .attendees
        .iter()
        .map(|person| format!("- {}", links::person(&person.name, style)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn notes(markdown: Option<&str>) -> String {
    let parsed = doc::demote(doc::parse(markdown.unwrap_or_default()), NOTE_DEMOTION);
    blocks::blocks(&parsed)
}

fn action_items(export: &MeetingExport, style: LinkStyle) -> String {
    export
        .action_items
        .iter()
        .map(|item| {
            let mark = if item.done { "x" } else { " " };
            let mut line = format!("- [{mark}] {}", blocks::escape(&item.title));
            let owner = item
                .assignee
                .as_deref()
                .map(|name| links::person(name, style));
            let due = item
                .due
                .map(|date| format!("due {}", labels::short_date(date)));
            let details: Vec<String> = owner.into_iter().chain(due).collect();
            if !details.is_empty() {
                line = format!("{line} — {}", details.join(", "));
            }
            if !item.description.trim().is_empty() {
                line = format!("{line}\n  {}", blocks::escape(item.description.trim()));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn transcript(export: &MeetingExport) -> String {
    let clock = Clock::for_transcript(&export.transcript);
    export
        .transcript
        .iter()
        .map(|line| {
            format!(
                "`{}` **{}:** {}",
                clock.format(line.start),
                blocks::escape(&line.speaker),
                blocks::escape(&line.text)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
