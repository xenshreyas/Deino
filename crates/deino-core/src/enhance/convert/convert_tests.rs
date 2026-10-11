//! Draft → TipTap: provenance on every run, citations after AI text, lists
//! grouped, and a document that passes `deino-doc`'s schema.

use deino_doc::{Author, Block, Flavor, Inline, Mark, to_markdown, validate};
use deino_domain::Millis;

use super::*;
use crate::enhance::test_support::segment;

fn evidence(reference: u32, seconds: u32) -> Evidence {
    Evidence {
        segment_id: segment(reference),
        start: Millis(seconds * 1_000),
    }
}

fn draft() -> Draft {
    Draft {
        sections: vec![
            DraftSection {
                heading: "sync v2".into(),
                heading_is_users: true,
                blocks: vec![
                    DraftBlock {
                        kind: DraftKind::Bullet,
                        body: Body::User(vec![
                            Inline::text("beta ", vec![]),
                            Inline::text("11/13", vec![Mark::Bold]),
                        ]),
                        evidence: vec![evidence(4, 271)],
                        details: vec![DraftDetail {
                            text: "GA holds at 4 December".into(),
                            evidence: vec![evidence(5, 280), evidence(9, 3_871)],
                        }],
                    },
                    DraftBlock {
                        kind: DraftKind::Bullet,
                        body: Body::Ai("Graph subscriptions expire in under three days".into()),
                        evidence: vec![],
                        details: vec![],
                    },
                ],
            },
            DraftSection {
                heading: "Action items".into(),
                heading_is_users: false,
                blocks: vec![DraftBlock {
                    kind: DraftKind::Todo { checked: true },
                    body: Body::User(vec![Inline::text("update pagerduty", vec![])]),
                    evidence: vec![],
                    details: vec![],
                }],
            },
        ],
    }
}

/// Every run's author, in document order.
fn authors(doc: &deino_doc::Doc) -> Vec<Author> {
    fn walk(block: &Block, out: &mut Vec<Author>) {
        for inline in block.inline_content() {
            for mark in inline.marks() {
                if let Mark::Origin { attrs } = mark {
                    out.push(attrs.author);
                }
            }
        }
        for child in block.children() {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    doc.content.iter().for_each(|block| walk(block, &mut out));
    out
}

#[test]
fn the_document_carries_origins_and_citations_and_validates() {
    let doc = to_doc(&draft());

    validate(&doc).unwrap();
    insta::assert_json_snapshot!(doc);
}

#[test]
fn every_run_has_an_origin_mark() {
    let doc = to_doc(&draft());

    let authors = authors(&doc);
    assert!(authors.contains(&Author::User));
    assert!(authors.contains(&Author::Ai));
    let runs: usize = doc.content.iter().map(count_inlines).sum();
    assert_eq!(authors.len(), runs, "no run without provenance");
}

fn count_inlines(block: &Block) -> usize {
    block.inline_content().len()
        + block
            .children()
            .into_iter()
            .map(count_inlines)
            .sum::<usize>()
}

#[test]
fn readable_markdown_shows_the_users_words_and_the_citation_times() {
    let markdown = to_markdown(&to_doc(&draft()), Flavor::Readable);

    insta::assert_snapshot!(markdown, @r"
    ### sync v2

    - beta **11/13** [4:31]
      - GA holds at 4 December [4:40][1:04:31]
    - Graph subscriptions expire in under three days

    ### Action items

    - [x] update pagerduty
    ");
}

#[test]
fn an_empty_draft_is_the_empty_document() {
    assert_eq!(to_doc(&Draft::default()), deino_doc::Doc::empty());
}
