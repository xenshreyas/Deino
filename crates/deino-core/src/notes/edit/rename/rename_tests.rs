//! The rename shortcut: which requests are renames, and what a rename
//! changes and leaves alone.

use deino_doc::{Block, Inline, Mark};

use super::*;
use crate::notes::edit::test_support::{
    NIALL, ai, ai_link, bullets, cite, doc, heading, mention, para, user,
};

fn rename(from: &str, to: &str) -> Rename {
    Rename {
        from: from.into(),
        to: to.into(),
    }
}

#[test]
fn plain_renames_are_recognized() {
    assert_eq!(
        Rename::parse("Niall is spelled Nyal"),
        Some(rename("Niall", "Nyal"))
    );
    assert_eq!(
        Rename::parse("niall is actually spelt Nyal."),
        Some(rename("niall", "Nyal"))
    );
    assert_eq!(
        Rename::parse("Rename Acme to Acme Corp everywhere"),
        Some(rename("Acme", "Acme Corp"))
    );
    assert_eq!(
        Rename::parse("replace \"Jon\" with “John” throughout"),
        Some(rename("Jon", "John"))
    );
    assert_eq!(
        Rename::parse("it's Nyal, not Niall"),
        Some(rename("Niall", "Nyal"))
    );
    assert_eq!(Rename::parse("Q3 → Q4"), Some(rename("Q3", "Q4")));
}

#[test]
fn other_requests_go_to_the_ai() {
    for request in [
        "make this more detailed",
        "remove the pricing section",
        "replace the pricing section with a short summary",
        "rename this heading to Costs",
        "change the tone to formal",
        "Niall is spelled Nyal and add the budget numbers to the summary please",
    ] {
        assert_eq!(Rename::parse(request), None, "{request}");
    }
}

/// Niall in a user line, a mention, AI text with a link, and a citation.
fn notes() -> deino_doc::Doc {
    doc(vec![
        heading("Niall's priorities"),
        bullets(vec![
            vec![user("ask "), mention("Niall"), user(" about pricing")],
            vec![
                ai("See "),
                ai_link("Niall's deck", "https://example.com/Niall"),
                ai(" for the plan "),
                cite(4),
            ],
        ]),
        para(vec![user("Nialls and niall-ish words stay; niall moves")]),
    ])
}

#[test]
fn every_occurrence_is_renamed_but_no_target_moves() {
    let (renamed, count) = rename("Niall", "Nyal").apply(&notes(), &Scope::whole());

    assert_eq!(count, 5);
    let markdown = deino_doc::to_markdown(&renamed, deino_doc::Flavor::Lossless);
    assert!(markdown.contains("Nyal's priorities"), "{markdown}");
    assert!(
        markdown.contains("Nyal's deck](https://example.com/Niall)"),
        "{markdown}"
    );
    assert!(
        markdown.contains("Nialls and Nyal-ish words stay; Nyal moves"),
        "{markdown}"
    );
    let inlines = all_inlines(&renamed);
    let mention = inlines.iter().find_map(|inline| match inline {
        Inline::Mention { attrs, .. } => Some(attrs.clone()),
        _ => None,
    });
    let mention = mention.unwrap();
    assert_eq!(
        (mention.label.as_str(), mention.id.to_string()),
        ("Nyal", NIALL.to_owned())
    );
    assert!(
        inlines.contains(&cite(4)),
        "the citation keeps its segment and time"
    );
    assert!(
        inlines
            .iter()
            .all(|inline| matches!(inline.marks().first(), Some(Mark::Origin { .. }))),
        "every run keeps its origin"
    );
}

#[test]
fn a_selection_limits_the_rename() {
    let scope = Scope::of(&[1], 4);
    let (renamed, count) = rename("Niall", "Nyal").apply(&notes(), &scope);

    assert_eq!(count, 1);
    let markdown = deino_doc::to_markdown(&renamed, deino_doc::Flavor::Readable);
    assert!(markdown.contains("Niall's priorities"), "{markdown}");
    assert!(markdown.contains("@Nyal"), "{markdown}");
}

#[test]
fn a_lower_case_name_keeps_the_capital_of_the_occurrence() {
    let (renamed, count) = rename("acme", "globex")
        .apply(&doc(vec![para(vec![ai("Acme and acme")])]), &Scope::whole());

    assert_eq!(count, 2);
    assert_eq!(
        deino_doc::to_markdown(&renamed, deino_doc::Flavor::Readable).trim(),
        "Globex and globex"
    );
}

fn all_inlines(doc: &deino_doc::Doc) -> Vec<Inline> {
    fn walk(blocks: &[Block], out: &mut Vec<Inline>) {
        for block in blocks {
            out.extend(block.inline_content().iter().cloned());
            let children: Vec<Block> = block.children().into_iter().cloned().collect();
            walk(&children, out);
        }
    }
    let mut out = Vec::new();
    walk(&doc.content, &mut out);
    out
}
