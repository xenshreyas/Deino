//! Rewrites keep the marks of kept words, the nodes they kept, and every
//! citation; new words are AI text.

use deino_doc::{Doc, Inline};

use super::*;
use crate::notes::edit::test_support::{ai, cite, line, mention, user};

fn refs() -> Refs {
    Refs::new(&[
        line(4, 4, "Nyal owns pricing."),
        line(7, 30, "Budget is 40k."),
    ])
}

fn normalized(content: Vec<Inline>) -> Vec<Inline> {
    let Doc { content: blocks } = Doc {
        content: vec![deino_doc::Block::paragraph(content)],
    }
    .normalized();
    blocks
        .into_iter()
        .flat_map(|block| block.inline_content().to_vec())
        .collect()
}

#[test]
fn the_prompt_shows_citations_as_references_and_mentions_by_name() {
    let content = [
        user("ask "),
        mention("Niall"),
        user(" about it "),
        cite(4),
        cite(9),
    ];

    assert_eq!(shown_text(&content, &refs()), "ask @Niall about it [s4]");
}

#[test]
fn kept_words_keep_their_origin_and_new_words_are_ai() {
    let original = [user("Niall owns pricing "), cite(4)];

    let content = normalized(rewrite(&original, "Nyal owns pricing [s4]", &refs()));

    assert_eq!(content, [ai("Nyal"), user(" owns pricing "), cite(4)]);
}

#[test]
fn a_dropped_citation_is_put_back_and_new_references_resolve() {
    let original = [ai("Pricing moves to Q3 "), cite(4)];

    let content = normalized(rewrite(
        &original,
        "Pricing moves to Q3 with a 40k budget [s7] [s99]",
        &refs(),
    ));

    let cited: Vec<&Inline> = content
        .iter()
        .filter(|inline| matches!(inline, Inline::Citation { .. }))
        .collect();
    assert_eq!(cited.len(), 2, "{content:?}");
    assert!(content.contains(&cite(4)));
    assert!(
        !content
            .iter()
            .any(|inline| matches!(inline, Inline::Text { text, .. } if text.contains("s99")))
    );
}

#[test]
fn a_kept_mention_stays_the_same_node() {
    let original = [user("ask "), mention("Niall"), user(" about pricing")];

    let content = normalized(rewrite(
        &original,
        "ask @Niall about pricing and budget",
        &refs(),
    ));

    assert_eq!(
        content,
        [
            user("ask "),
            mention("Niall"),
            user(" about pricing"),
            ai(" and budget")
        ]
    );
}
