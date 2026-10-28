use chrono::DateTime;
use deino_domain::{FakeClock, Millis, SegmentId};

use super::*;
use crate::enhance::wire::{BlockKind, EnhancedBlock};

fn block(origin: Origin, text: &str) -> EnhancedBlock {
    EnhancedBlock {
        kind: BlockKind::Bullet,
        text: text.to_owned(),
        origin,
        evidence: vec!["s1".to_owned()],
        details: Vec::new(),
    }
}

fn doc(sections: &[(&str, Vec<EnhancedBlock>)]) -> EnhancedDoc {
    EnhancedDoc {
        sections: sections
            .iter()
            .map(|(heading, blocks)| EnhancedSection {
                heading: (*heading).to_owned(),
                blocks: blocks.clone(),
            })
            .collect(),
    }
}

fn line(reference: u32) -> TranscriptLine {
    TranscriptLine {
        reference,
        segment_id: SegmentId::generate(&FakeClock::new(DateTime::UNIX_EPOCH)),
        start: Millis(reference * 1_000),
        end: Millis(reference * 1_000 + 500),
        speaker: "Speaker 1".to_owned(),
        text: "words".to_owned(),
    }
}

#[test]
fn parts_merge_by_heading_in_order_keeping_a_typed_line_once() {
    let first = doc(&[
        (
            "Decisions",
            vec![block(Origin::User, "ship it"), block(Origin::Ai, "a")],
        ),
        ("Next steps", vec![block(Origin::Ai, "b")]),
    ]);
    let second = doc(&[
        (
            "decisions ",
            vec![block(Origin::User, "ship it"), block(Origin::Ai, "c")],
        ),
        ("Risks", vec![block(Origin::Ai, "d")]),
    ]);
    let lines = [line(1)];

    let merged = merge_locally(&[(&lines[..], first), (&lines[..], second)]);

    let shape: Vec<(String, Vec<String>)> = merged
        .sections
        .iter()
        .map(|s| {
            (
                s.heading.clone(),
                s.blocks.iter().map(|b| b.text.clone()).collect(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            (
                "Decisions".to_owned(),
                vec!["ship it".to_owned(), "a".to_owned(), "c".to_owned()]
            ),
            ("Next steps".to_owned(), vec!["b".to_owned()]),
            ("Risks".to_owned(), vec!["d".to_owned()]),
        ]
    );
}

#[test]
fn only_an_unusable_answer_falls_back_or_splits() {
    assert!(is_answer_failure(&LlmError::Truncated));
    assert!(is_answer_failure(&LlmError::InvalidOutput {
        detail: String::new()
    }));
    assert!(!is_answer_failure(&LlmError::Cancelled));

    let lines = [line(1)];
    let noted = [(&lines[..], doc(&[("Notes", vec![block(Origin::Ai, "x")])]))];
    let kept = or_merge_locally(Err(LlmError::Truncated), &noted).unwrap();
    assert_eq!(kept.sections.len(), 1);
    assert!(matches!(
        or_merge_locally(Err(LlmError::Cancelled), &noted),
        Err(LlmError::Cancelled)
    ));
}

#[test]
fn a_transcript_is_halved_only_when_there_is_something_to_split() {
    let lines: Vec<TranscriptLine> = (1..=5).map(line).collect();
    let halves = halve(&lines);
    assert_eq!(halves.iter().map(|h| h.len()).collect::<Vec<_>>(), [2, 3]);
    assert!(halve(&lines[..1]).is_empty());
}
