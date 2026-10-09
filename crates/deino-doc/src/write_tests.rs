//! Doc → Markdown: the exact text each flavor writes.

use crate::test_support::*;
use crate::{Block, Doc, Flavor, Mark, to_markdown};

/// A note with every node and mark, shaped like a NOT-02 enhanced note.
fn kitchen_sink() -> Doc {
    let mut content = vec![
        h(1, vec![t("Q4 roadmap")]),
        p(vec![
            t("Agreed with "),
            mention("Priya", vec![]),
            t(" to "),
            m("narrow", vec![Mark::Bold]),
            t(" Q3."),
        ]),
    ];
    content.extend(lists());
    content.extend([
        Block::Blockquote {
            content: vec![p(vec![t("ship it"), br(), m("— Maya", vec![Mark::Italic])])],
        },
        Block::HorizontalRule,
        image("images/0192f3a0.png", Some("whiteboard"), None),
        p(vec![
            m("run", vec![Mark::Code]),
            t(" then "),
            m("docs", vec![link("https://a.io/docs")]),
        ]),
    ]);
    doc(content)
}

/// The kitchen sink's three kinds of list.
fn lists() -> Vec<Block> {
    vec![
        bullets(vec![
            vec![
                p(vec![m("push notifications first", vec![user()])]),
                bullets(vec![vec![p(vec![
                    m("retention fell 40% without push ", vec![ai()]),
                    citation(3_724_000, vec![ai()]),
                ])]]),
            ],
            vec![p(vec![m("SMB paused", vec![ai_edited(), Mark::Italic])])],
        ]),
        numbered(
            1,
            vec![
                vec![p(vec![t("scope doc")])],
                vec![p(vec![m("design", vec![Mark::Underline])])],
            ],
        ),
        tasks(vec![
            (true, vec![p(vec![t("book room")])]),
            (false, vec![p(vec![t("send deck")])]),
        ]),
    ]
}

#[test]
fn lossless_markdown_carries_every_node_and_mark() {
    insta::assert_snapshot!(
        "kitchen_sink_lossless",
        to_markdown(&kitchen_sink(), Flavor::Lossless)
    );
}

#[test]
fn readable_markdown_is_plain_for_people_and_llms() {
    insta::assert_snapshot!(
        "kitchen_sink_readable",
        to_markdown(&kitchen_sink(), Flavor::Readable)
    );
}

#[test]
fn a_blank_document_writes_nothing() {
    assert_eq!(to_markdown(&Doc::empty(), Flavor::Readable), "");
}

#[test]
fn only_syntax_is_escaped() {
    let written = to_markdown(
        &doc(vec![p(vec![t(
            "1. snake_case ~19d <7d Q&A *a* _b_ [c] # d",
        )])]),
        Flavor::Lossless,
    );
    assert_eq!(
        written,
        "1\\. snake_case ~19d <7d Q&A \\*a\\* \\_b\\_ \\[c\\] # d\n"
    );
}

#[test]
fn whitespace_moves_outside_emphasis() {
    let written = to_markdown(
        &doc(vec![p(vec![t("a"), m(" bold ", vec![Mark::Bold]), t("b")])]),
        Flavor::Lossless,
    );
    assert_eq!(written, "a **bold** b\n");
}

#[test]
fn code_spans_outgrow_their_backticks() {
    let written = to_markdown(
        &doc(vec![p(vec![m("a`b", vec![Mark::Code])])]),
        Flavor::Lossless,
    );
    assert_eq!(written, "``a`b``\n");
}

#[test]
fn citations_read_as_timestamps() {
    let readable = |ms| to_markdown(&doc(vec![p(vec![citation(ms, vec![])])]), Flavor::Readable);
    assert_eq!(readable(64_000), "[1:04]\n");
    assert_eq!(readable(3_724_000), "[1:02:04]\n");
}
