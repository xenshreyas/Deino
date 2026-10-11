//! The user's lines: what counts as one, and how copies are matched.

use deino_doc::{Inline, Mark, from_markdown};

use super::*;

fn lines(markdown: &str) -> Vec<UserLine> {
    user_lines(&from_markdown(markdown).unwrap())
}

fn texts(lines: &[UserLine]) -> Vec<&str> {
    lines.iter().map(|line| line.text.as_str()).collect()
}

#[test]
fn every_bullet_todo_and_paragraph_line_is_a_line_and_headings_are_not() {
    let lines = lines(
        "## maya 1:1\n\nv tired\nslept 11h\n\n- OFF oncall 2 wks\n  - update pagerduty!!\n\n* [ ] send PM draft thurs\n* [x] book room",
    );

    assert_eq!(
        texts(&lines),
        [
            "v tired",
            "slept 11h",
            "OFF oncall 2 wks",
            "update pagerduty!!",
            "send PM draft thurs",
            "book room"
        ]
    );
    let kinds: Vec<LineKind> = lines.iter().map(|line| line.kind).collect();
    assert_eq!(
        kinds,
        [
            LineKind::Paragraph,
            LineKind::Paragraph,
            LineKind::Bullet,
            LineKind::Bullet,
            LineKind::Todo { checked: false },
            LineKind::Todo { checked: true },
        ]
    );
}

#[test]
fn a_line_keeps_its_formatting_but_not_any_provenance() {
    let doc = deino_doc::Doc {
        content: vec![deino_doc::Block::paragraph(vec![
            Inline::text(
                "capacity => ",
                vec![Mark::Origin {
                    attrs: deino_doc::OriginAttrs::USER,
                }],
            ),
            Inline::text("30 eng-wks", vec![Mark::Bold]),
        ])],
    };

    let lines = user_lines(&doc);

    assert_eq!(lines[0].text, "capacity => 30 eng-wks");
    assert_eq!(lines[0].content[0].marks(), &[]);
    assert_eq!(lines[0].content[1].marks(), &[Mark::Bold]);
}

#[test]
fn headings_are_kept_apart_as_structure() {
    let doc = from_markdown("## maya 1:1\n\n- a\n\n### sync v2\n\n- b").unwrap();

    assert_eq!(user_headings(&doc), ["maya 1:1", "sync v2"]);
}

#[test]
fn a_copy_matches_its_line_ignoring_case_markers_and_emphasis() {
    let lines = lines("- capacity => **30 eng-wks**\n- DECISIONS: v2, soc2");
    let mut matcher = LineMatcher::new(&lines);

    assert_eq!(matcher.claim("- Capacity => 30 eng-wks"), Some(0));
    assert_eq!(matcher.claim("decisions:  v2, soc2"), Some(1));
    assert!(matcher.unclaimed().is_empty());
}

#[test]
fn a_near_copy_matches_but_unrelated_text_and_second_claims_do_not() {
    let lines = lines("- maya postmortem thu\n- freeze 18 dec - 4 jan");
    let mut matcher = LineMatcher::new(&lines);

    assert_eq!(matcher.claim("maya postmortem on thu"), Some(0));
    assert_eq!(
        matcher.claim("maya postmortem thu"),
        None,
        "each line is claimed once"
    );
    assert_eq!(matcher.claim("The team discussed the freeze window"), None);
    assert_eq!(
        matcher.claim_exact("freeze 18 dec"),
        None,
        "exact claims never take a near copy"
    );
    assert_eq!(matcher.unclaimed(), [1]);
}

#[test]
fn word_overlap_is_shared_words_over_all_words() {
    assert!((word_overlap("a b c", "a b c") - 1.0).abs() < f64::EPSILON);
    assert!((word_overlap("a b", "b c") - 1.0 / 3.0).abs() < 1e-9);
    assert!(word_overlap("", "").abs() < f64::EPSILON);
}
