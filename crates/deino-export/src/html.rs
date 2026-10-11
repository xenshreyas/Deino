//! The meeting as an HTML fragment: the rich-text flavour of "Copy as rich
//! text" (with [`text`](crate::text) as its plain-text fallback) and the
//! export dialog's document preview.
//!
//! The fragment uses plain tags and a few inline styles, because mail and
//! document editors drop `<style>` blocks and classes on paste. Secondary
//! text carries `data-role="meta"` so a preview can recolour it per theme.

mod blocks;

use blocks::{MUTED, escape};

use crate::labels::{self, Clock, heading};
use crate::{MeetingExport, RenderOptions, Section, doc};

/// The fragment's font stack: the system UI font, then Inter, then the web's.
const FONT: &str = "-apple-system,BlinkMacSystemFont,'Inter',Helvetica,Arial,sans-serif";

/// The meeting as an HTML fragment.
pub fn render(export: &MeetingExport, options: &RenderOptions) -> String {
    let mut body = header(export);
    for section in export.sections(options.sections) {
        body.push_str(&section_body(export, section));
    }
    format!("<div style=\"font-family:{FONT};color:#1F1F1C;line-height:1.5\">{body}</div>")
}

fn header(export: &MeetingExport) -> String {
    let mut meta = labels::long_date(&export.started_at);
    if let Some(duration) = export.duration {
        meta = format!("{meta} · {}", labels::duration(duration));
    }
    format!(
        "<h1 style=\"font-size:22px;font-weight:600;margin:0 0 4px\">{}</h1>{}",
        escape(&export.title),
        meta_paragraph(&meta)
    )
}

fn meta_paragraph(text: &str) -> String {
    format!(
        "<p data-role=\"meta\" style=\"{MUTED};margin:0 0 16px\">{}</p>",
        escape(text)
    )
}

fn section_body(export: &MeetingExport, section: Section) -> String {
    let (title, body) = match section {
        Section::Attendees => (heading::ATTENDEES, attendees(export)),
        Section::Notes => (heading::NOTES, notes(export.notes.as_deref())),
        Section::MyNotes => (heading::MY_NOTES, notes(export.my_notes.as_deref())),
        Section::ActionItems => (heading::ACTION_ITEMS, action_items(export)),
        Section::Transcript => (heading::TRANSCRIPT, transcript(export)),
        Section::Insights => return String::new(),
    };
    format!("<h2 style=\"font-size:16px;font-weight:600;margin:20px 0 8px\">{title}</h2>{body}")
}

fn attendees(export: &MeetingExport) -> String {
    let names: Vec<&str> = export
        .attendees
        .iter()
        .map(|person| person.name.as_str())
        .collect();
    format!("<p>{}</p>", escape(&names.join(", ")))
}

fn notes(markdown: Option<&str>) -> String {
    let parsed = doc::demote(doc::parse(markdown.unwrap_or_default()), 2);
    blocks::blocks(&parsed)
}

fn action_items(export: &MeetingExport) -> String {
    let items = export
        .action_items
        .iter()
        .map(|item| {
            let mark = if item.done { "☑" } else { "☐" };
            let details: Vec<String> = item
                .assignee
                .clone()
                .into_iter()
                .chain(
                    item.due
                        .map(|date| format!("due {}", labels::short_date(date))),
                )
                .collect();
            let meta = if details.is_empty() {
                String::new()
            } else {
                format!(
                    " <span data-role=\"meta\" style=\"{MUTED}\">— {}</span>",
                    escape(&details.join(", "))
                )
            };
            format!("<li>{mark} {}{meta}</li>", escape(&item.title))
        })
        .collect::<Vec<_>>()
        .concat();
    format!("<ul style=\"list-style:none;padding-left:4px\">{items}</ul>")
}

fn transcript(export: &MeetingExport) -> String {
    let clock = Clock::for_transcript(&export.transcript);
    export
        .transcript
        .iter()
        .map(|line| {
            format!(
                "<p><span data-role=\"meta\" style=\"{MUTED};font-variant-numeric:tabular-nums\">{}</span> <strong>{}</strong> {}</p>",
                clock.format(line.start),
                escape(&line.speaker),
                escape(&line.text)
            )
        })
        .collect::<Vec<_>>()
        .concat()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExportSections;
    use crate::test_support::fixture_export;

    #[test]
    fn the_shared_notes_render_as_a_fragment() {
        let options = RenderOptions {
            sections: ExportSections::SHARE,
            ..RenderOptions::default()
        };
        insta::assert_snapshot!(
            "html_one_on_one",
            render(&fixture_export("one-on-one"), &options)
        );
    }

    #[test]
    fn meeting_text_cannot_inject_markup() {
        let mut export = fixture_export("standup");
        export.title = "<script>x</script>".into();
        export.transcript.truncate(1);
        export.transcript[0].speaker = "<b>Eve</b>".into();
        let options = RenderOptions {
            sections: ExportSections::ALL,
            ..RenderOptions::default()
        };

        let html = render(&export, &options);

        assert!(!html.contains("<script>"));
        assert!(!html.contains("<b>Eve"));
        assert!(html.contains("&lt;script&gt;x&lt;/script&gt;"));
    }
}
