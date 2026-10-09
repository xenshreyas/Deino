//! Every node and mark survives Doc → Lossless Markdown → Doc unchanged, and
//! real notes are stable from their second conversion on.

use std::path::Path;

use crate::test_support::*;
use crate::{
    Block, CodeBlockAttrs, Doc, Flavor, LinkAttrs, Mark, from_markdown, to_markdown, validate,
};

/// Asserts `doc` is valid and comes back equal from its Lossless Markdown.
fn assert_round_trip(doc: &Doc) {
    validate(doc).unwrap();
    let markdown = to_markdown(doc, Flavor::Lossless);
    let back = from_markdown(&markdown).unwrap();
    assert_eq!(&back, doc, "the Markdown was:\n{markdown}");
}

#[test]
fn paragraphs_round_trip() {
    assert_round_trip(&doc(vec![
        p(vec![t("Hello, world.")]),
        p(vec![t("Second paragraph")]),
    ]));
}

#[test]
fn headings_of_every_level_round_trip() {
    let headings = (1..=6)
        .map(|level| h(level, vec![t(&format!("Level {level}"))]))
        .collect();
    assert_round_trip(&doc(headings));
}

#[test]
fn bullet_lists_nest() {
    assert_round_trip(&doc(vec![bullets(vec![
        vec![p(vec![t("one")]), bullets(vec![vec![p(vec![t("one.a")])]])],
        vec![p(vec![t("two")])],
    ])]));
}

#[test]
fn ordered_lists_keep_their_start() {
    assert_round_trip(&doc(vec![numbered(
        3,
        vec![vec![p(vec![t("third")])], vec![p(vec![t("fourth")])]],
    )]));
    assert_round_trip(&doc(vec![numbered(1, vec![vec![p(vec![t("first")])]])]));
}

#[test]
fn task_lists_keep_their_state_and_nesting() {
    assert_round_trip(&doc(vec![tasks(vec![
        (false, vec![p(vec![t("send the deck")])]),
        (
            true,
            vec![
                p(vec![t("book the room")]),
                tasks(vec![(false, vec![p(vec![t("nested")])])]),
            ],
        ),
    ])]));
}

#[test]
fn neighbouring_lists_stay_apart() {
    let item = |text: &str| vec![p(vec![t(text)])];
    assert_round_trip(&doc(vec![
        bullets(vec![item("a")]),
        bullets(vec![item("b")]),
        tasks(vec![(false, item("c"))]),
        numbered(1, vec![item("d")]),
        numbered(1, vec![item("e")]),
    ]));
    assert_round_trip(&doc(vec![bullets(vec![vec![
        p(vec![t("parent")]),
        bullets(vec![item("x")]),
        tasks(vec![(true, item("y"))]),
    ]])]));
}

#[test]
fn list_items_hold_more_than_one_paragraph() {
    assert_round_trip(&doc(vec![bullets(vec![vec![
        p(vec![t("first")]),
        p(vec![t("second, same item")]),
    ]])]));
}

#[test]
fn blockquotes_round_trip() {
    assert_round_trip(&doc(vec![Block::Blockquote {
        content: vec![
            p(vec![t("quoted")]),
            bullets(vec![vec![p(vec![t("in a quote")])]]),
        ],
    }]));
}

#[test]
fn code_blocks_keep_their_language_and_backticks() {
    assert_round_trip(&doc(vec![Block::CodeBlock {
        attrs: CodeBlockAttrs {
            language: Some("rust".into()),
        },
        content: vec![t("fn main() {\n    let fence = \"```\";\n}")],
    }]));
}

#[test]
fn horizontal_rules_and_images_round_trip() {
    assert_round_trip(&doc(vec![
        p(vec![t("above")]),
        Block::HorizontalRule,
        image(
            "images/0192f3a0.png",
            Some("the [whiteboard]"),
            Some("a \"title\""),
        ),
        image("images/no alt (1).png", None, None),
    ]));
}

#[test]
fn hard_breaks_round_trip() {
    assert_round_trip(&doc(vec![p(vec![
        t("line one"),
        br(),
        t("line two"),
        br(),
        m("bold", vec![Mark::Bold]),
    ])]));
}

#[test]
fn every_formatting_mark_round_trips() {
    for mark in [
        Mark::Bold,
        Mark::Italic,
        Mark::Underline,
        Mark::Strike,
        Mark::Code,
        link("https://example.com/a_b"),
    ] {
        assert_round_trip(&doc(vec![p(vec![
            t("before "),
            m("marked", vec![mark]),
            t(" after"),
        ])]));
    }
}

#[test]
fn links_keep_their_title_and_awkward_targets() {
    let titled = Mark::Link {
        attrs: LinkAttrs {
            href: "https://example.com/a (b)".into(),
            title: Some("say \"hi\"".into()),
        },
    };
    assert_round_trip(&doc(vec![p(vec![
        m("link", vec![titled]),
        t(" and "),
        m("mail", vec![link("mailto:a@b.co")]),
    ])]));
}

#[test]
fn nested_and_overlapping_marks_round_trip() {
    assert_round_trip(&doc(vec![p(vec![
        m("bold ", vec![Mark::Bold]),
        m("bold italic", vec![Mark::Bold, Mark::Italic]),
        m(" bold", vec![Mark::Bold]),
        t(" plain "),
        m("linked ", vec![link("https://a.io")]),
        m("bold link", vec![link("https://a.io"), Mark::Bold]),
        t(" "),
        m("struck", vec![Mark::Underline, Mark::Strike]),
    ])]));
}

#[test]
fn origin_marks_round_trip_for_user_ai_and_edited_ai() {
    // Whitespace inside an origin span is kept exactly, unlike at a `**` edge.
    assert_round_trip(&doc(vec![p(vec![
        m("typed by me ", vec![user()]),
        m(" written by the AI ", vec![ai()]),
        m("with", vec![ai(), Mark::Bold]),
        m(" ", vec![ai()]),
        m("code", vec![ai(), Mark::Code]),
        t(" and "),
        m("edited", vec![ai_edited(), Mark::Italic]),
    ])]));
}

#[test]
fn mentions_round_trip_with_marks_and_awkward_labels() {
    assert_round_trip(&doc(vec![
        p(vec![t("ask "), mention("Priya", vec![]), t(" about it")]),
        p(vec![mention("Jordan \"JE\" <Ellis> & co", vec![ai()])]),
        p(vec![
            m("bold ", vec![Mark::Bold]),
            mention("Priya", vec![Mark::Bold]),
        ]),
    ]));
}

#[test]
fn citations_round_trip() {
    assert_round_trip(&doc(vec![
        p(vec![
            m("retention fell 40% without push", vec![ai()]),
            citation(724_000, vec![ai()]),
        ]),
        p(vec![citation(0, vec![])]),
    ]));
}

#[test]
fn text_that_looks_like_markdown_stays_text() {
    let lookalikes = [
        "# not a heading",
        "> not a quote",
        "- not a list",
        "+ nor this",
        "1. not numbered",
        "2) nor this",
        "--- not a rule",
        "=== not a heading",
        "*not emphasis*",
        "_nor this_",
        "snake_case_name",
        "a ~~strike~~ b",
        "~19 days",
        "[not](a link)",
        "`not code`",
        "a <div> tag",
        "<7 days",
        "&amp; stays",
        "Q&A",
        "back\\slash",
        "100% | done",
        "!not an image",
    ];
    for text in lookalikes {
        assert_round_trip(&doc(vec![p(vec![t(text)])]));
        assert_round_trip(&doc(vec![p(vec![t("first"), br(), t(text)])]));
    }
}

#[test]
fn a_realistic_enhanced_note_round_trips() {
    assert_round_trip(&doc(vec![
        h(1, vec![m("Overview", vec![ai()])]),
        bullets(vec![
            vec![
                p(vec![m("narrow Q3 to mid-market", vec![user()])]),
                bullets(vec![vec![p(vec![
                    m("SMB deprioritised for Q3", vec![ai()]),
                    citation(64_000, vec![ai()]),
                ])]]),
            ],
            vec![p(vec![
                m("push notifications first, owner ", vec![ai_edited()]),
                mention("Priya", vec![ai_edited()]),
            ])],
        ]),
        h(2, vec![m("Action items", vec![ai()])]),
        tasks(vec![(
            false,
            vec![p(vec![m("send the roadmap by Friday", vec![ai()])])],
        )]),
    ]));
}

/// The fixture meetings' raw notes, as typed in other note-takers.
fn fixture_notes() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/meetings");
    let mut notes = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        if let Some(text) = json["userNotes"].as_str() {
            notes.push((path.display().to_string(), text.to_owned()));
        }
    }
    assert!(
        notes.len() >= 5,
        "expected the fixtures to carry user notes"
    );
    notes
}

#[test]
fn fixture_notes_are_stable_through_markdown() {
    for (path, markdown) in fixture_notes() {
        let first = from_markdown(&markdown).unwrap();
        validate(&first).unwrap();
        let written = to_markdown(&first, Flavor::Lossless);
        assert_eq!(
            from_markdown(&written).unwrap(),
            first,
            "{path}:\n{written}"
        );
        assert_eq!(
            to_markdown(&from_markdown(&written).unwrap(), Flavor::Lossless),
            written,
            "{path}"
        );
    }
}
