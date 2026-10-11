//! Plain text: the TXT export, the clipboard's plain-text fallback and the
//! body of an email draft. No markup at all, so it reads the same anywhere.

mod blocks;

use crate::labels::{self, Clock, heading};
use crate::{MeetingExport, RenderOptions, Section, doc};

/// The meeting as plain text.
pub fn render(export: &MeetingExport, options: &RenderOptions) -> String {
    let mut parts = vec![header(export)];
    for section in export.sections(options.sections) {
        if let Some(body) = section_body(export, section) {
            parts.push(body);
        }
    }
    let mut text = parts.join("\n\n");
    text.push('\n');
    text
}

/// The title and "Thursday, October 8, 2026 at 15:31 · 7 min".
fn header(export: &MeetingExport) -> String {
    let mut meta = labels::long_date(&export.started_at);
    if let Some(duration) = export.duration {
        meta = format!("{meta} · {}", labels::duration(duration));
    }
    format!("{}\n{meta}", export.title)
}

fn section_body(export: &MeetingExport, section: Section) -> Option<String> {
    let (title, body) = match section {
        Section::Attendees => (heading::ATTENDEES, attendees(export)),
        Section::Notes => (heading::NOTES, notes(export.notes.as_deref())),
        Section::MyNotes => (heading::MY_NOTES, notes(export.my_notes.as_deref())),
        Section::ActionItems => (heading::ACTION_ITEMS, action_items(export)),
        Section::Transcript => (heading::TRANSCRIPT, transcript(export)),
        Section::Insights => return None,
    };
    let rule = "-".repeat(title.chars().count());
    Some(format!("{title}\n{rule}\n{body}"))
}

fn attendees(export: &MeetingExport) -> String {
    export
        .attendees
        .iter()
        .map(|person| person.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn notes(markdown: Option<&str>) -> String {
    blocks::blocks(&doc::parse(markdown.unwrap_or_default()))
}

fn action_items(export: &MeetingExport) -> String {
    export
        .action_items
        .iter()
        .map(|item| {
            let mark = if item.done { "[x]" } else { "[ ]" };
            let details: Vec<String> = item
                .assignee
                .clone()
                .into_iter()
                .chain(
                    item.due
                        .map(|date| format!("due {}", labels::short_date(date))),
                )
                .collect();
            let mut line = format!("{mark} {}", item.title);
            if !details.is_empty() {
                line = format!("{line} — {}", details.join(", "));
            }
            if !item.description.trim().is_empty() {
                line = format!("{line}\n    {}", item.description.trim());
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
                "[{}] {}: {}",
                clock.format(line.start),
                line.speaker,
                line.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fixture_export, fixture_exports};
    use crate::{ExportSections, LinkStyle};

    #[test]
    fn every_fixture_renders_as_plain_text() {
        for export in fixture_exports() {
            let name = export
                .id
                .as_str()
                .trim_start_matches("meeting-fixture-")
                .to_owned();
            insta::assert_snapshot!(
                format!("text_{name}"),
                render(&export, &RenderOptions::default())
            );
        }
    }

    #[test]
    fn the_standup_transcript_reads_as_lines() {
        let options = RenderOptions {
            sections: ExportSections::ALL,
            links: LinkStyle::Plain,
        };
        let text = render(&fixture_export("standup"), &options);

        insta::assert_snapshot!("text_standup_all", text);
        assert!(text.contains("\n[00:00] Shreyas: Morning, everyone."));
    }
}
