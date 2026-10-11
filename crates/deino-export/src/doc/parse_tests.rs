use super::*;

fn text(value: &str) -> Inline {
    Inline::text(value)
}

fn paragraph(value: &str) -> Block {
    Block::Paragraph {
        content: vec![text(value)],
    }
}

#[test]
fn headings_paragraphs_and_rules_become_blocks() {
    let blocks = parse("# Overview\n\nWe shipped.\n\n---\n");

    assert_eq!(
        blocks,
        [
            Block::Heading {
                level: 1,
                content: vec![text("Overview")]
            },
            paragraph("We shipped."),
            Block::Rule,
        ]
    );
}

#[test]
fn tight_list_items_hold_their_text_as_a_paragraph() {
    let blocks = parse("- one\n- two\n  - nested\n");

    let Block::List { ordered, items } = &blocks[0] else {
        panic!("a list: {blocks:?}");
    };
    assert!(!ordered);
    assert_eq!(items[0].blocks, [paragraph("one")]);
    assert_eq!(items[1].blocks[0], paragraph("two"));
    assert!(matches!(items[1].blocks[1], Block::List { .. }));
}

#[test]
fn task_list_markers_set_the_item_state() {
    let blocks = parse("- [ ] open\n- [x] done\n- plain\n");

    let Block::List { items, .. } = &blocks[0] else {
        panic!("a list: {blocks:?}");
    };
    let states: Vec<Option<bool>> = items.iter().map(|item| item.checked).collect();
    assert_eq!(states, [Some(false), Some(true), None]);
    assert_eq!(items[1].blocks, [paragraph("done")]);
}

#[test]
fn inline_formatting_nests() {
    let blocks = parse("a **bold _both_** ~~gone~~ `code`");

    assert_eq!(
        blocks,
        [Block::Paragraph {
            content: vec![
                text("a "),
                Inline::Strong {
                    content: vec![
                        text("bold "),
                        Inline::Emphasis {
                            content: vec![text("both")]
                        },
                    ]
                },
                text(" "),
                Inline::Strike {
                    content: vec![text("gone")]
                },
                text(" "),
                Inline::Code {
                    text: "code".into()
                },
            ]
        }]
    );
}

#[test]
fn soft_breaks_are_spaces_and_hard_breaks_are_kept() {
    let blocks = parse("one\ntwo  \nthree");

    assert_eq!(
        blocks,
        [Block::Paragraph {
            content: vec![text("one two"), Inline::Break, text("three")]
        }]
    );
}

#[test]
fn raw_html_stays_text_and_code_blocks_keep_their_lines() {
    let blocks = parse("<script>alert(1)</script>\n\n```\nlet a = 1;\nlet b = 2;\n```\n");

    assert_eq!(
        blocks,
        [
            paragraph("<script>alert(1)</script>\n"),
            Block::Code {
                text: "let a = 1;\nlet b = 2;".into()
            },
        ]
    );
}

#[test]
fn quotes_and_links_keep_their_structure() {
    let blocks = parse("> see [the doc](https://deino.dev/doc)\n");

    assert_eq!(
        blocks,
        [Block::Quote {
            blocks: vec![Block::Paragraph {
                content: vec![
                    text("see "),
                    Inline::Link {
                        href: "https://deino.dev/doc".into(),
                        content: vec![text("the doc")],
                    },
                ]
            }]
        }]
    );
}

#[test]
fn empty_notes_have_no_blocks() {
    assert!(parse("").is_empty());
    assert!(parse("   \n\n").is_empty());
}
