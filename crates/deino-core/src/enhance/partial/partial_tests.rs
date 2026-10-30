//! `complete_lines`: what a cut-off answer shows, and the prefix property.

use proptest::prelude::*;

use super::*;
use crate::enhance::wire::{BlockKind, Origin};

const ANSWER: &str = r#"{"sections":[{"heading":"Overview","blocks":[
  {"kind":"bullet","text":"Pilot starts \"Nov 10\"","origin":"ai","evidence":["s4"]},
  {"kind":"todo","text":"send the questionnaire","origin":"user","evidence":[],
   "details":[{"text":"By Friday","evidence":["s9"]},{"text":"To Maya","evidence":[]}]}
]},{"heading":"Decisions","blocks":[{"kind":"paragraph","text":"Six weeks","origin":"ai"}]}]}"#;

fn shown(text: &str) -> Vec<String> {
    let doc = complete_lines(text);
    let mut lines = Vec::new();
    for section in &doc.sections {
        lines.push(format!("# {}", section.heading));
        for block in &section.blocks {
            lines.push(format!(
                "{:?}/{:?}: {}",
                block.kind, block.origin, block.text
            ));
            lines.extend(
                block
                    .details
                    .iter()
                    .map(|detail| format!("- {}", detail.text)),
            );
        }
    }
    lines
}

#[test]
fn a_finished_answer_shows_every_line_without_evidence() {
    let doc = complete_lines(ANSWER);
    let mut expected: EnhancedDoc = serde_json::from_str(ANSWER).unwrap();
    for block in expected
        .sections
        .iter_mut()
        .flat_map(|s| s.blocks.iter_mut())
    {
        block.evidence.clear();
        block
            .details
            .iter_mut()
            .for_each(|detail| detail.evidence.clear());
    }

    assert_eq!(doc, expected);
    assert_eq!(line_count(&doc), 7);
}

#[test]
fn a_block_shows_once_its_origin_is_known() {
    let cut = ANSWER.find(r#""origin":"ai""#).unwrap();

    assert_eq!(shown(&ANSWER[..cut]), ["# Overview"]);
    assert_eq!(
        shown(&ANSWER[..cut + r#""origin":"ai""#.len()]),
        ["# Overview", "Bullet/Ai: Pilot starts \"Nov 10\""]
    );
}

#[test]
fn a_heading_shows_once_it_is_closed_and_half_a_detail_never_does() {
    let heading = ANSWER.find("Decisions").unwrap();
    let detail = ANSWER.find("To Ma").unwrap();

    assert!(!shown(&ANSWER[..heading + 4]).contains(&"# Decisions".to_owned()));
    assert!(shown(&ANSWER[..heading + "Decisions\"".len()]).contains(&"# Decisions".to_owned()));
    assert_eq!(shown(&ANSWER[..detail + 5]).last().unwrap(), "- By Friday");
}

#[test]
fn nothing_or_garbage_shows_nothing() {
    assert!(complete_lines("").sections.is_empty());
    assert!(
        complete_lines("Here are the notes: {\"sec")
            .sections
            .is_empty()
    );
    assert!(complete_lines("not json at all").sections.is_empty());
}

#[test]
fn field_order_does_not_matter() {
    let answer =
        r#"{"sections":[{"blocks":[{"origin":"user","kind":"bullet","text":"x"}],"heading":"H"}]}"#;

    assert_eq!(shown(answer), ["# H", "Bullet/User: x"]);
}

fn text() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 \"\\\\é—\n]{0,12}"
}

fn kind() -> impl Strategy<Value = BlockKind> {
    prop_oneof![
        Just(BlockKind::Bullet),
        Just(BlockKind::Paragraph),
        Just(BlockKind::Todo)
    ]
}

fn origin() -> impl Strategy<Value = Origin> {
    prop_oneof![Just(Origin::Ai), Just(Origin::User)]
}

fn block() -> impl Strategy<Value = EnhancedBlock> {
    let detail = (text(), prop::collection::vec("s[0-9]{1,2}", 0..2))
        .prop_map(|(text, evidence)| EnhancedDetail { text, evidence });
    (
        kind(),
        text(),
        origin(),
        prop::collection::vec("s[0-9]{1,2}", 0..3),
        prop::collection::vec(detail, 0..3),
    )
        .prop_map(|(kind, text, origin, evidence, details)| EnhancedBlock {
            kind,
            text,
            origin,
            evidence,
            details,
        })
}

fn doc() -> impl Strategy<Value = EnhancedDoc> {
    let section = (text(), prop::collection::vec(block(), 0..4))
        .prop_map(|(heading, blocks)| EnhancedSection { heading, blocks });
    prop::collection::vec(section, 0..4).prop_map(|sections| EnhancedDoc { sections })
}

/// `part`'s lines are where `whole`'s are, and only the last can be shorter.
fn is_prefix(part: &[String], whole: &[String]) -> bool {
    part.len() <= whole.len() && part.iter().zip(whole).all(|(a, b)| a == b)
}

proptest! {
    #[test]
    fn every_prefix_of_an_answer_shows_a_prefix_of_its_lines(doc in doc(), pretty in any::<bool>()) {
        let answer = if pretty {
            serde_json::to_string_pretty(&doc).unwrap()
        } else {
            serde_json::to_string(&doc).unwrap()
        };
        let whole = shown(&answer);
        prop_assert_eq!(whole.len(), line_count(&doc));
        let mut previous = 0;
        for (at, _) in answer.char_indices() {
            let part = shown(&answer[..at]);
            prop_assert!(is_prefix(&part, &whole), "cut at {at}: {part:?} vs {whole:?}");
            prop_assert!(part.len() >= previous, "lines never disappear");
            previous = part.len();
        }
    }
}
