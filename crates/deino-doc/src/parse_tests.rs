//! Markdown → Doc: what the parser makes of Markdown from people and LLMs,
//! including the constructs the editor has no node for.

use crate::test_support::*;
use crate::{Block, Doc, Mark, from_markdown, validate};

fn parse(markdown: &str) -> Doc {
    let doc = from_markdown(markdown).unwrap();
    validate(&doc).unwrap();
    doc
}

#[test]
fn empty_markdown_is_the_empty_document() {
    assert_eq!(parse(""), Doc::empty());
    assert_eq!(parse("  \n\n"), Doc::empty());
}

#[test]
fn soft_line_breaks_keep_the_authors_lines() {
    assert_eq!(
        parse("first line  \nsecond line\nthird"),
        doc(vec![p(vec![
            t("first line"),
            br(),
            t("second line"),
            br(),
            t("third")
        ])])
    );
}

#[test]
fn nested_emphasis_keeps_each_runs_marks() {
    assert_eq!(
        parse("**bold *both* bold** and ~~gone~~"),
        doc(vec![p(vec![
            m("bold ", vec![Mark::Bold]),
            m("both", vec![Mark::Bold, Mark::Italic]),
            m(" bold", vec![Mark::Bold]),
            t(" and "),
            m("gone", vec![Mark::Strike]),
        ])])
    );
}

#[test]
fn gfm_task_lists_become_task_lists() {
    assert_eq!(
        parse("- [ ] open\n- [x] done\n  - [ ] nested"),
        doc(vec![tasks(vec![
            (false, vec![p(vec![t("open")])]),
            (
                true,
                vec![
                    p(vec![t("done")]),
                    tasks(vec![(false, vec![p(vec![t("nested")])])])
                ]
            ),
        ])])
    );
}

#[test]
fn a_list_item_that_starts_with_a_list_gets_an_empty_paragraph_first() {
    assert_eq!(
        parse("- - inner"),
        doc(vec![bullets(vec![vec![
            p(vec![]),
            bullets(vec![vec![p(vec![t("inner")])]])
        ]])])
    );
}

#[test]
fn images_are_lifted_out_of_their_paragraph() {
    assert_eq!(
        parse("before ![chart](images/a.png) after\n\n![](images/b.png)"),
        doc(vec![
            p(vec![t("before ")]),
            image("images/a.png", Some("chart"), None),
            p(vec![t(" after")]),
            image("images/b.png", None, None),
        ])
    );
}

#[test]
fn unsafe_links_keep_their_text_without_the_link() {
    assert_eq!(
        parse("[click](javascript:alert(1)) or [file](file:///etc/passwd) or [ok](https://a.io)"),
        doc(vec![p(vec![
            t("click or file or "),
            m("ok", vec![link("https://a.io")]),
        ])])
    );
}

#[test]
fn bare_urls_stay_text() {
    assert_eq!(
        parse("see https://example.com"),
        doc(vec![p(vec![t("see https://example.com")])])
    );
}

#[test]
fn tables_become_one_paragraph_per_row() {
    assert_eq!(
        parse("| Owner | Task |\n|---|---|\n| **Priya** | spec |"),
        doc(vec![
            p(vec![t("Owner | Task")]),
            p(vec![m("Priya", vec![Mark::Bold]), t(" | spec")]),
        ])
    );
}

#[test]
fn unknown_html_stays_literal_and_br_breaks_the_line() {
    assert_eq!(
        parse("a <kbd>K</kbd> key<br>next"),
        doc(vec![p(vec![t("a <kbd>K</kbd> key"), br(), t("next")])])
    );
}

#[test]
fn mention_and_citation_tags_with_bad_ids_stay_literal() {
    let markdown = r#"<mention data-id="nope" data-label="X" /> <citation data-segment-id="1" data-start-ms="x" />"#;
    let Block::Paragraph { content } = &parse(markdown).content[0] else {
        panic!("expected a paragraph");
    };
    assert!(
        content
            .iter()
            .all(|inline| matches!(inline, crate::Inline::Text { .. }))
    );
}

#[test]
fn an_unclosed_origin_span_ends_with_its_block() {
    assert_eq!(
        parse("<span data-origin=\"ai\">open\n\nnext"),
        doc(vec![p(vec![m("open", vec![ai()])]), p(vec![t("next")])])
    );
}

#[test]
fn headings_code_quotes_and_rules() {
    assert_eq!(
        parse("## Title\n\n> quoted\n\n---\n\n```sql\nselect 1;\n```"),
        doc(vec![
            h(2, vec![t("Title")]),
            Block::Blockquote {
                content: vec![p(vec![t("quoted")])],
            },
            Block::HorizontalRule,
            Block::CodeBlock {
                attrs: crate::CodeBlockAttrs {
                    language: Some("sql".into()),
                },
                content: vec![t("select 1;")],
            },
        ])
    );
}

#[test]
fn the_wire_format_is_tiptap_json() {
    let doc = parse(
        "# Plan\n\n- [x] **ship** it\n\nsee <mention data-id=\"0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e\" data-label=\"Priya\" />",
    );
    insta::assert_json_snapshot!("wire_format", doc);
}

#[test]
fn llm_markdown_with_extras_still_parses() {
    let markdown = "Intro text.\n\n1. First\n2. Second\n\n* star bullet\n+ plus bullet\n\nTerm[^1]\n\n[^1]: a footnote\n\n[ref]: https://a.io";
    let doc = parse(markdown);
    insta::assert_json_snapshot!("llm_extras", doc);
}
