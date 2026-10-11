//! What the PDF template reads: the meeting reduced to labelled sections,
//! serialized as `data.json` into the typesetter's world.
//!
//! Meeting text only ever travels as JSON strings, which the template
//! places as text; it is never parsed as Typst markup, so nothing a meeting
//! says can change the document.

use serde::Serialize;

use crate::doc::{self, Block};
use crate::labels::{self, Clock, heading};
use crate::{MeetingExport, RenderOptions, Section};

/// How many speaker colours the template has (ui.md §2.2 speaker palette).
const SPEAKER_COLOURS: usize = 6;

/// The template's input.
#[derive(Serialize)]
pub(crate) struct PrintModel {
    title: String,
    meta: String,
    tags: Vec<String>,
    sections: Vec<PrintSection>,
}

/// One section of the document.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum PrintSection {
    Names {
        heading: &'static str,
        names: Vec<String>,
    },
    Notes {
        heading: &'static str,
        blocks: Vec<Block>,
    },
    Tasks {
        heading: &'static str,
        items: Vec<Task>,
    },
    Transcript {
        heading: &'static str,
        lines: Vec<Line>,
    },
}

#[derive(Serialize)]
struct Task {
    title: String,
    details: String,
    done: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Line {
    time: String,
    speaker: String,
    /// Index into the speaker palette, by order of first appearance.
    colour: usize,
    /// True when the previous line has the same speaker, whose name is
    /// then not repeated.
    continued: bool,
    text: String,
}

impl PrintModel {
    /// The model of `export` with the chosen sections.
    pub(crate) fn new(export: &MeetingExport, options: RenderOptions) -> Self {
        let mut meta = labels::long_date(&export.started_at);
        if let Some(duration) = export.duration {
            meta = format!("{meta} · {}", labels::duration(duration));
        }
        Self {
            title: export.title.clone(),
            meta,
            tags: export.tags.clone(),
            sections: export
                .sections(options.sections)
                .into_iter()
                .filter_map(|section| section_of(export, section))
                .collect(),
        }
    }
}

fn section_of(export: &MeetingExport, section: Section) -> Option<PrintSection> {
    let notes = |markdown: Option<&str>| doc::demote(doc::parse(markdown.unwrap_or_default()), 1);
    Some(match section {
        Section::Attendees => PrintSection::Names {
            heading: heading::ATTENDEES,
            names: export
                .attendees
                .iter()
                .map(|person| person.name.clone())
                .collect(),
        },
        Section::Notes => PrintSection::Notes {
            heading: heading::NOTES,
            blocks: notes(export.notes.as_deref()),
        },
        Section::MyNotes => PrintSection::Notes {
            heading: heading::MY_NOTES,
            blocks: notes(export.my_notes.as_deref()),
        },
        Section::ActionItems => PrintSection::Tasks {
            heading: heading::ACTION_ITEMS,
            items: tasks(export),
        },
        Section::Transcript => PrintSection::Transcript {
            heading: heading::TRANSCRIPT,
            lines: lines(export),
        },
        Section::Insights => return None,
    })
}

fn tasks(export: &MeetingExport) -> Vec<Task> {
    export
        .action_items
        .iter()
        .map(|item| Task {
            title: item.title.clone(),
            details: item
                .assignee
                .clone()
                .into_iter()
                .chain(
                    item.due
                        .map(|date| format!("due {}", labels::short_date(date))),
                )
                .collect::<Vec<_>>()
                .join(" · "),
            done: item.done,
        })
        .collect()
}

fn lines(export: &MeetingExport) -> Vec<Line> {
    let clock = Clock::for_transcript(&export.transcript);
    let mut speakers: Vec<&str> = Vec::new();
    let mut previous: Option<&str> = None;
    export
        .transcript
        .iter()
        .map(|line| {
            let index = speakers
                .iter()
                .position(|name| *name == line.speaker)
                .unwrap_or_else(|| {
                    speakers.push(&line.speaker);
                    speakers.len() - 1
                });
            let continued = previous == Some(line.speaker.as_str());
            previous = Some(&line.speaker);
            Line {
                time: clock.format(line.start),
                speaker: line.speaker.clone(),
                colour: index % SPEAKER_COLOURS,
                continued,
                text: line.text.clone(),
            }
        })
        .collect()
}
