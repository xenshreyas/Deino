//! The checks between the model's answer and the notes: the user's lines
//! kept word for word, provenance enforced, citations validated.

use deino_doc::{Inline, Mark};

use super::*;
use crate::enhance::test_support::{answer, block, line, section, with_details};
use crate::enhance::transcript::TranscriptLine;
use crate::enhance::user_lines::{user_headings, user_lines};

const NOTES: &str = "## sync v2\n\n- beta 11/13, ga 12/4 - holding\n- MSFT graph subs expire ~3 days\n- [x] update **pagerduty** today!!\n\nunified cal api vendor - prob not";

fn transcript() -> Vec<TranscriptLine> {
    (1..=6).map(|n| line(n, n * 10, "Maya", "text")).collect()
}

fn run(model: &EnhancedDoc) -> Draft {
    let doc = deino_doc::from_markdown(NOTES).unwrap();
    let (lines, headings) = (user_lines(&doc), user_headings(&doc));
    let index = SegmentIndex::new(&transcript());
    assemble(
        model,
        Checks {
            lines: &lines,
            headings: &headings,
            index: &index,
        },
    )
}

fn user_text(body: &Body) -> String {
    match body {
        Body::User(content) => crate::enhance::user_lines::plain_text(content),
        Body::Ai(text) => panic!("expected the user's line, got AI text {text:?}"),
    }
}

#[test]
fn a_quoted_user_line_is_rendered_from_the_users_own_document() {
    let draft = run(&answer(vec![section(
        "Sync v2",
        vec![with_details(
            block(
                BlockKind::Bullet,
                Origin::User,
                "beta 11/13, ga 12/4 - holding",
                &["s1"],
            ),
            &[("Google push is fine; the dates hold.", &["s2", "s404"])],
        )],
    )]));

    let first = &draft.sections[0].blocks[0];
    assert_eq!(user_text(&first.body), "beta 11/13, ga 12/4 - holding");
    assert_eq!(first.evidence.len(), 1);
    assert_eq!(
        first.details[0].evidence.len(),
        1,
        "the invented s404 is dropped"
    );
}

#[test]
fn a_reworded_user_line_is_restored_to_the_users_words() {
    let draft = run(&answer(vec![section(
        "Sync v2",
        vec![block(
            BlockKind::Bullet,
            Origin::User,
            "MSFT Graph subs expire ~3 days",
            &[],
        )],
    )]));

    assert_eq!(
        user_text(&draft.sections[0].blocks[0].body),
        "MSFT graph subs expire ~3 days"
    );
}

#[test]
fn a_user_todo_keeps_its_tick_and_its_formatting() {
    let draft = run(&answer(vec![section(
        "Action items",
        vec![block(
            BlockKind::Bullet,
            Origin::User,
            "update pagerduty today!!",
            &[],
        )],
    )]));

    let todo = &draft.sections[0].blocks[0];
    assert_eq!(todo.kind, DraftKind::Todo { checked: true });
    let Body::User(content) = &todo.body else {
        panic!("a user block")
    };
    assert!(content.iter().any(|inline| matches!(inline, Inline::Text { text, marks } if text == "pagerduty" && marks == &[Mark::Bold])));
}

#[test]
fn text_the_model_calls_the_users_but_the_user_never_typed_is_ai_text() {
    let draft = run(&answer(vec![section(
        "Overview",
        vec![block(
            BlockKind::Bullet,
            Origin::User,
            "The team agreed to delay the launch",
            &["s3"],
        )],
    )]));

    assert_eq!(
        draft.sections[0].blocks[0].body,
        Body::Ai("The team agreed to delay the launch".into())
    );
}

#[test]
fn ai_text_that_is_exactly_a_user_line_is_the_users() {
    let draft = run(&answer(vec![section(
        "Vendors",
        vec![block(
            BlockKind::Paragraph,
            Origin::Ai,
            "unified cal api vendor - prob not",
            &[],
        )],
    )]));

    assert_eq!(
        user_text(&draft.sections[0].blocks[0].body),
        "unified cal api vendor - prob not"
    );
}

#[test]
fn every_user_line_the_model_dropped_is_put_back() {
    let draft = run(&answer(vec![section(
        "Sync v2 dates",
        vec![block(
            BlockKind::Bullet,
            Origin::Ai,
            "Beta and GA dates hold; graph subscriptions expire",
            &["s1"],
        )],
    )]));

    let placed: Vec<String> = draft.sections[0].blocks[1..]
        .iter()
        .map(|b| user_text(&b.body))
        .collect();
    assert!(placed.contains(&"beta 11/13, ga 12/4 - holding".to_owned()));
    let leftovers = draft
        .sections
        .iter()
        .find(|s| s.heading == LEFTOVER_HEADING)
        .unwrap();
    let texts: Vec<String> = leftovers
        .blocks
        .iter()
        .map(|b| user_text(&b.body))
        .collect();
    assert!(texts.contains(&"update pagerduty today!!".to_owned()));
}

#[test]
fn a_users_heading_is_recognised_and_model_noise_is_cleaned() {
    let draft = run(&answer(vec![
        section(
            "sync v2",
            vec![block(
                BlockKind::Bullet,
                Origin::Ai,
                "- **Renewal** job every 6h [s4, s5]",
                &["s4"],
            )],
        ),
        section(
            "Empty",
            vec![block(BlockKind::Bullet, Origin::Ai, "  ", &[])],
        ),
    ]));

    assert!(draft.sections[0].heading_is_users);
    assert_eq!(
        draft.sections[0].blocks[0].body,
        Body::Ai("Renewal job every 6h".into())
    );
    assert!(
        draft.sections.iter().all(|s| s.heading != "Empty"),
        "empty sections go"
    );
}
