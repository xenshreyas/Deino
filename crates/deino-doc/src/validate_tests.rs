//! The schema rules, accepted and rejected, plus the JSON the editor sends.
//! The structural cases are ported from anarlog's `validate.rs` tests.

use serde_json::json;

use crate::test_support::*;
use crate::{
    Block, Doc, DocError, Inline, ListItem, Mark, OriginAttrs, TaskItem, TaskItemAttrs, validate,
};

fn issues(doc: &Doc) -> Vec<String> {
    match validate(doc) {
        Ok(()) => vec![],
        Err(DocError::Invalid(issues)) => issues.iter().map(ToString::to_string).collect(),
        Err(other) => panic!("unexpected error {other}"),
    }
}

fn assert_rejected(doc: &Doc, fragment: &str) {
    let found = issues(doc);
    assert!(
        found.iter().any(|issue| issue.contains(fragment)),
        "expected an issue containing {fragment:?}, got {found:?}"
    );
}

#[test]
fn valid_documents_pass() {
    for doc in [
        Doc::empty(),
        doc(vec![bullets(vec![vec![
            p(vec![t("item")]),
            bullets(vec![vec![p(vec![])]]),
        ]])]),
        doc(vec![p(vec![m("code", vec![ai(), Mark::Code])])]),
        doc(vec![p(vec![m("both", vec![Mark::Bold, Mark::Italic])])]),
    ] {
        assert_eq!(issues(&doc), Vec::<String>::new());
    }
}

#[test]
fn structure_rules_are_enforced() {
    assert_rejected(&doc(vec![]), "at least one block");
    assert_rejected(
        &doc(vec![Block::BulletList { content: vec![] }]),
        "at least one item",
    );
    assert_rejected(
        &doc(vec![Block::BulletList {
            content: vec![ListItem {
                content: vec![bullets(vec![vec![p(vec![])]])],
            }],
        }]),
        "must start with a paragraph",
    );
    assert_rejected(
        &doc(vec![Block::TaskList {
            content: vec![TaskItem {
                attrs: TaskItemAttrs { checked: false },
                content: vec![],
            }],
        }]),
        "must start with a paragraph",
    );
    assert_rejected(
        &doc(vec![Block::Blockquote { content: vec![] }]),
        "at least one block",
    );
    assert_rejected(
        &doc(vec![h(7, vec![t("too deep")])]),
        "level must be 1 to 6",
    );
    assert_rejected(&doc(vec![image(" ", None, None)]), "needs a source");
}

#[test]
fn inline_rules_are_enforced() {
    assert_rejected(&doc(vec![p(vec![t("")])]), "cannot be empty");
    assert_rejected(&doc(vec![p(vec![mention(" ", vec![])])]), "needs a label");
    assert_rejected(
        &doc(vec![p(vec![m("x", vec![Mark::Bold, Mark::Code])])]),
        "code combines",
    );
    assert_rejected(
        &doc(vec![p(vec![m("x", vec![Mark::Bold, Mark::Bold])])]),
        "appears twice",
    );
    assert_rejected(
        &doc(vec![p(vec![m("x", vec![link("javascript:alert(1)")])])]),
        "http, https or mailto",
    );
    let edited_user = Mark::Origin {
        attrs: OriginAttrs {
            edited: true,
            ..OriginAttrs::USER
        },
    };
    assert_rejected(
        &doc(vec![p(vec![m("x", vec![edited_user])])]),
        "only AI text",
    );
    let code = Block::CodeBlock {
        attrs: crate::CodeBlockAttrs::default(),
        content: vec![m("x", vec![Mark::Bold])],
    };
    assert_rejected(&doc(vec![code]), "unformatted text only");
}

#[test]
fn issues_name_their_path() {
    let nested = doc(vec![
        p(vec![t("ok")]),
        bullets(vec![vec![p(vec![t("ok"), Inline::text("", vec![])])]]),
    ]);
    assert_eq!(
        issues(&nested),
        ["at content[1].content[0].content[0].content[1]: a text node cannot be empty"]
    );
}

#[test]
fn editor_json_with_extra_attributes_deserializes() {
    // What TipTap 3's getJSON() emits: defaults Deino does not model are ignored.
    let json = json!({
        "type": "doc",
        "content": [
            { "type": "paragraph" },
            { "type": "orderedList", "attrs": { "start": 1, "type": null }, "content": [
                { "type": "listItem", "content": [{ "type": "paragraph", "content": [
                    { "type": "text", "text": "link", "marks": [{ "type": "link", "attrs": {
                        "href": "https://a.io", "target": "_blank", "rel": "noopener", "class": null, "title": null } }] },
                    { "type": "mention", "attrs": { "id": PRIYA, "label": "Priya", "mentionSuggestionChar": "@" } }
                ] }] }
            ] },
            { "type": "image", "attrs": { "src": "images/a.png", "alt": null, "title": null, "width": null, "height": null } }
        ]
    });
    let doc: Doc = serde_json::from_value(json).unwrap();
    assert_eq!(issues(&doc), Vec::<String>::new());
    assert_eq!(doc.images().len(), 1);
}

#[test]
fn json_that_breaks_the_types_is_refused() {
    let unknown_node = json!({ "type": "doc", "content": [{ "type": "table" }] });
    let bad_person = json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [
        { "type": "mention", "attrs": { "id": "not-a-uuid", "label": "X" } }] }] });
    assert!(serde_json::from_value::<Doc>(unknown_node).is_err());
    assert!(serde_json::from_value::<Doc>(bad_person).is_err());
}

#[test]
fn blankness_and_normalization() {
    assert!(doc(vec![p(vec![t("  "), br()]), Block::HorizontalRule]).is_blank());
    assert!(!doc(vec![p(vec![mention("Priya", vec![])])]).is_blank());
    let messy = doc(vec![p(vec![
        m("a", vec![Mark::Italic, Mark::Bold]),
        m("b", vec![Mark::Bold, Mark::Italic]),
    ])]);
    assert_eq!(
        messy.normalized(),
        doc(vec![p(vec![m("ab", vec![Mark::Bold, Mark::Italic])])])
    );
}
