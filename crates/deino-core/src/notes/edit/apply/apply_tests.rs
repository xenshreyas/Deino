//! Plans applied to a document: deletes, inserts and replacements land on
//! the blocks the model named, and nothing else moves.

use deino_doc::{Flavor, to_markdown};

use super::*;
use crate::notes::edit::test_support::{ai, bullets, doc, heading, para, user};

/// b1 Pricing, b2–b3 its bullets, b4 Next steps, b5 a paragraph.
fn notes() -> Doc {
    doc(vec![
        heading("Pricing"),
        bullets(vec![vec![user("price is 10")], vec![ai("Discount of 5%")]]),
        heading("Next steps"),
        para(vec![ai("Ship it on Friday.")]),
    ])
}

fn readable(doc: &Doc) -> String {
    to_markdown(doc, Flavor::Readable)
}

#[test]
fn deleting_a_section_removes_its_heading_and_list() {
    let plan = Plan {
        delete: [0, 1, 2].into(),
        ..Plan::default()
    };

    let edited = apply(&notes(), &plan);

    assert_eq!(readable(&edited), "### Next steps\n\nShip it on Friday.\n");
}

#[test]
fn a_block_inserted_after_a_list_item_joins_the_list() {
    let plan = Plan {
        insert: [(
            1,
            vec![NewBlock {
                kind: NewKind::Paragraph,
                content: vec![ai("Annual plans get 2 months free")],
            }],
        )]
        .into(),
        ..Plan::default()
    };

    let edited = apply(&notes(), &plan);

    assert!(
        readable(&edited)
            .contains("- price is 10\n- Annual plans get 2 months free\n- Discount of 5%"),
        "{}",
        readable(&edited)
    );
}

#[test]
fn a_replaced_heading_stays_a_heading_and_the_rest_is_untouched() {
    let plan = Plan {
        replace: [(3, vec![ai("Follow-ups")])].into(),
        insert: [(
            4,
            vec![NewBlock {
                kind: NewKind::Todo,
                content: vec![ai("Send the deck")],
            }],
        )]
        .into(),
        ..Plan::default()
    };

    let edited = apply(&notes(), &plan);

    let original = notes();
    assert_eq!(edited.content.first(), original.content.first());
    assert_eq!(edited.content.get(1), original.content.get(1));
    assert!(
        readable(&edited)
            .ends_with("### Follow-ups\n\nShip it on Friday.\n\n- [ ] Send the deck\n")
    );
}

#[test]
fn deleting_everything_leaves_an_empty_document() {
    let plan = Plan {
        delete: (0..5).collect(),
        ..Plan::default()
    };

    assert_eq!(apply(&notes(), &plan), Doc::empty());
}
