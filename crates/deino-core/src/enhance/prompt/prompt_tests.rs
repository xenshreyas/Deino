//! Snapshots of the rendered `notes.enhance` prompts: the meeting, template,
//! notes and transcript blocks, a part, and the merge with its references.

use super::*;
use crate::enhance::material::PreviousNotes;
use crate::enhance::test_support::{
    answer, block, line, material, one_on_one, section, with_details,
};
use crate::enhance::wire::{BlockKind, Origin};
use deino_testkit::at;

const NOTES: &str = "- v tired post fernhollow\n- [ ] update pagerduty today!!";

fn lines() -> Vec<TranscriptLine> {
    vec![
        line(1, 4, "Shreyas", "How are you holding up after Fernhollow?"),
        line(2, 9, "Maya Chen", "Tired, honestly. I slept eleven hours."),
        line(3, 31, "Shreyas", "Then you're off on-call for two weeks."),
    ]
}

#[test]
fn one_pass_renders_every_input() {
    let mut material = material(Some(NOTES), lines());
    material.previous = Some(PreviousNotes {
        title: "Maya 1:1".into(),
        date: at("2026-09-30T15:00:00Z"),
        markdown: "### Updates\n\n- beta on track".into(),
    });
    material.vocabulary = vec!["Fernhollow".into(), "PagerDuty".into()];

    let prompt = PromptInputs::new(&material, &one_on_one()).single(&material.transcript);

    insta::assert_snapshot!(prompt.messages[0].text());
    assert_eq!(prompt.max_tokens, Some(8_192));
    assert!(prompt.system.unwrap().contains("character for character"));
}

#[test]
fn a_meeting_without_transcript_is_written_from_the_notes() {
    let material = material(Some(NOTES), Vec::new());

    let prompt = PromptInputs::new(&material, &one_on_one()).single(&[]);

    assert!(
        prompt.messages[0]
            .text()
            .contains("write the notes from the user's notes alone")
    );
}

#[test]
fn a_part_names_its_span_and_carries_only_its_lines() {
    let material = material(Some(NOTES), lines());

    let prompt = PromptInputs::new(&material, &one_on_one()).part(2, 3, &material.transcript[1..]);
    let text = prompt.messages[0].text();

    assert!(text.contains(r#"<transcript part="2 of 3" span="00:09–00:35">"#));
    assert!(!text.contains("[s1 "));
    assert!(
        prompt
            .system
            .unwrap()
            .contains("one part of a long meeting")
    );
}

#[test]
fn the_merge_keeps_valid_references_and_drops_invented_ones() {
    let material = material(Some(NOTES), lines());
    let index = SegmentIndex::new(&material.transcript);
    let first = answer(vec![section(
        "Wellbeing",
        vec![with_details(
            block(
                BlockKind::Bullet,
                Origin::Ai,
                "Maya is exhausted after the incident",
                &["s2", "s99"],
            ),
            &[("Slept eleven hours", &["s2"])],
        )],
    )]);
    let second = answer(vec![section(
        "Action items",
        vec![block(
            BlockKind::Todo,
            Origin::Ai,
            "Take Maya off on-call",
            &["s3"],
        )],
    )]);
    let parts = [
        (&material.transcript[..2], first),
        (&material.transcript[2..], second),
    ];

    let prompt = PromptInputs::new(&material, &one_on_one()).merge(&parts, &index);

    insta::assert_snapshot!(prompt.messages[0].text());
}
