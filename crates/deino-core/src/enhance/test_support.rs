//! Builders shared by this module's tests: transcript lines, material, a
//! template and model answers.

use deino_domain::{MeetingId, Millis, SegmentId, TemplateId};
use deino_testkit::at;

use super::material::{AttendeeInfo, Material, MeetingInfo, UserNotes};
use super::transcript::TranscriptLine;
use super::wire::{BlockKind, EnhancedBlock, EnhancedDetail, EnhancedDoc, EnhancedSection, Origin};
use crate::templates::{CATALOG, Template};

pub(super) const MEETING: &str = "meeting-1on1";

/// A stable segment ID for line `reference`.
pub(super) fn segment(reference: u32) -> SegmentId {
    uuid::Uuid::from_u128(0x0192_f3a0_6c1e_7b2a_9d4e_0000_0000_0000 + u128::from(reference))
        .to_string()
        .parse()
        .unwrap()
}

/// Line `reference`, starting at `seconds`, four seconds long.
pub(super) fn line(reference: u32, seconds: u32, speaker: &str, text: &str) -> TranscriptLine {
    TranscriptLine {
        reference,
        segment_id: segment(reference),
        start: Millis(seconds * 1_000),
        end: Millis(seconds * 1_000 + 4_000),
        speaker: speaker.into(),
        text: text.into(),
    }
}

/// A 1:1 with `notes` (Markdown, if any) and `lines`.
pub(super) fn material(notes: Option<&str>, lines: Vec<TranscriptLine>) -> Material {
    Material {
        meeting: MeetingInfo {
            id: MeetingId::new(MEETING),
            title: "Maya 1:1".into(),
            date: at("2026-10-07T15:00:00Z"),
            calendar_title: Some("Maya / Shreyas weekly".into()),
            description: "Weekly one-on-one.".into(),
            attendees: vec![
                AttendeeInfo {
                    name: "Shreyas".into(),
                    title: Some("CTO".into()),
                    is_me: true,
                },
                AttendeeInfo {
                    name: "Maya Chen".into(),
                    title: Some("Staff engineer".into()),
                    is_me: false,
                },
            ],
            series_id: None,
        },
        notes: notes.map(|markdown| UserNotes {
            doc: deino_doc::from_markdown(markdown).unwrap(),
            markdown: markdown.into(),
            updated_at: at("2026-10-07T15:20:00Z"),
        }),
        transcript: lines,
        transcript_edited_at: None,
        attendee_ids: Vec::new(),
        previous: None,
        vocabulary: Vec::new(),
    }
}

/// The built-in 1:1 template.
pub(super) fn one_on_one() -> Template {
    let builtin = CATALOG
        .iter()
        .find(|template| template.id == "one_on_one")
        .unwrap();
    Template {
        id: TemplateId::new(builtin.id),
        is_builtin: true,
        content: builtin.content(),
        created_at: at("2026-10-01T00:00:00Z"),
        updated_at: at("2026-10-01T00:00:00Z"),
    }
}

/// A block of `kind` by `origin`, citing `evidence`.
pub(super) fn block(
    kind: BlockKind,
    origin: Origin,
    text: &str,
    evidence: &[&str],
) -> EnhancedBlock {
    EnhancedBlock {
        kind,
        text: text.into(),
        origin,
        evidence: evidence.iter().map(ToString::to_string).collect(),
        details: Vec::new(),
    }
}

/// `block` with AI `details` (text, evidence).
pub(super) fn with_details(mut block: EnhancedBlock, details: &[(&str, &[&str])]) -> EnhancedBlock {
    block.details = details
        .iter()
        .map(|(text, evidence)| EnhancedDetail {
            text: (*text).into(),
            evidence: evidence.iter().map(ToString::to_string).collect(),
        })
        .collect();
    block
}

/// One section.
pub(super) fn section(heading: &str, blocks: Vec<EnhancedBlock>) -> EnhancedSection {
    EnhancedSection {
        heading: heading.into(),
        blocks,
    }
}

/// An answer of `sections`.
pub(super) fn answer(sections: Vec<EnhancedSection>) -> EnhancedDoc {
    EnhancedDoc { sections }
}
