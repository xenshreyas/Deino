//! Meetings as Markdown: the list and the full read.

use deino_core::query::{AttendeeView, MeetingDetail, MeetingSummary};
use deino_domain::MeetingStatus;

use super::items::item_line;
use super::{joined, length, line, nested, when};
use crate::request::{Markdown, MeetingDetails, MeetingList};

impl Markdown for MeetingList {
    fn markdown(&self) -> String {
        let mut out = String::new();
        if self.meetings.is_empty() {
            line(&mut out, "No meetings match.");
            return out;
        }
        line(&mut out, format!("# Meetings ({})\n", self.meetings.len()));
        for meeting in &self.meetings {
            line(
                &mut out,
                format!("- **{}** · {}", meeting.title, facts(meeting)),
            );
            if let Some(details) = who_and_tags(meeting) {
                line(&mut out, format!("  {details}"));
            }
        }
        out
    }
}

impl Markdown for MeetingDetails {
    fn markdown(&self) -> String {
        let pages: Vec<String> = self.meetings.iter().map(detail).collect();
        pages.join("\n---\n\n")
    }
}

/// Date, length, status and ID on one line.
fn facts(meeting: &MeetingSummary) -> String {
    let mut parts = vec![when(&meeting.held_at)];
    if let Some(seconds) = meeting.duration_s {
        parts.push(length(seconds));
    }
    if meeting.status != MeetingStatus::Ready {
        parts.push(meeting.status.as_str().to_owned());
    }
    parts.push(format!("`{}`", meeting.id));
    parts.join(" · ")
}

fn who_and_tags(meeting: &MeetingSummary) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(people) = joined(&meeting.attendees) {
        parts.push(people);
    }
    if let Some(tags) = joined(&meeting.tags) {
        parts.push(format!("tags: {tags}"));
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn detail(meeting: &MeetingDetail) -> String {
    let summary = &meeting.summary;
    let mut out = String::new();
    line(&mut out, format!("# {}\n", summary.title));
    line(&mut out, format!("{} · {}", facts(summary), summary.link));
    if !meeting.people.is_empty() {
        let people: Vec<String> = meeting.people.iter().map(person).collect();
        line(&mut out, format!("Attendees: {}", people.join(", ")));
    }
    if let Some(tags) = joined(&summary.tags) {
        line(&mut out, format!("Tags: {tags}"));
    }
    if let Some(folders) = joined(&meeting.folders) {
        line(&mut out, format!("Folders: {folders}"));
    }
    section(&mut out, "AI notes", meeting.ai_notes.as_deref());
    cited(&mut out, meeting);
    section(&mut out, "My notes", meeting.my_notes.as_deref());
    action_items(&mut out, meeting);
    files(&mut out, meeting);
    out
}

fn person(attendee: &AttendeeView) -> String {
    let me = if attendee.is_me { " (me)" } else { "" };
    match &attendee.email {
        Some(email) => format!("{}{me} <{email}>", attendee.name),
        None => format!("{}{me}", attendee.name),
    }
}

fn section(out: &mut String, title: &str, markdown: Option<&str>) {
    let Some(text) = markdown.filter(|text| !text.trim().is_empty()) else {
        return;
    };
    line(out, format!("\n## {title}\n"));
    line(out, nested(text.trim_end()));
}

/// Where each `[12:04]` in the AI notes points.
fn cited(out: &mut String, meeting: &MeetingDetail) {
    if meeting.citations.is_empty() {
        return;
    }
    let links: Vec<String> = meeting
        .citations
        .iter()
        .map(|citation| format!("[{}]({})", citation.label, citation.link))
        .collect();
    line(out, format!("\nCited moments: {}", links.join(" · ")));
}

fn action_items(out: &mut String, meeting: &MeetingDetail) {
    if meeting.action_items.is_empty() {
        return;
    }
    line(out, "\n## Action items\n");
    for item in &meeting.action_items {
        line(out, item_line(item, false));
    }
}

fn files(out: &mut String, meeting: &MeetingDetail) {
    line(out, "\n## Recording\n");
    let audio = &meeting.audio;
    for (label, path) in [
        ("Audio", &audio.mixed),
        ("Microphone", &audio.microphone),
        ("System audio", &audio.system),
    ] {
        if let Some(path) = path {
            line(out, format!("- {label}: {path}"));
        }
    }
    line(
        out,
        format!(
            "- Transcript: {} segments (get_transcript, or `deino transcripts read {}`)",
            meeting.transcript_segments, meeting.summary.id
        ),
    );
}
