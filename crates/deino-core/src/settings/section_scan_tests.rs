//! Tests of the section scan that generates [`Settings`](super::Settings)
//! (`build/settings.rs`, compiled here as well as into `build.rs`).

#[path = "../../build/settings.rs"]
mod scan;

use std::path::Path;

use scan::{camel, parse_file, read};

const REGISTRATION: &str = r#"
pub struct ExportSettings;

crate::settings::register_section! {
    /// Exports and auto-export (EXP-02).
    ///
    /// A second paragraph.
    export: ExportSettings,
    order: 140,
    validate: ExportSettings::validate,
    dialog_only: ["folder", "otherFolder"],
}
"#;

#[test]
fn a_registration_is_read_line_by_line() {
    let sections = parse_file(REGISTRATION).unwrap();

    let [section] = sections.as_slice() else {
        panic!("one section expected");
    };
    assert_eq!(section.key, "export");
    assert_eq!(section.ty, "ExportSettings");
    assert_eq!(section.order, 140);
    assert_eq!(
        section.doc,
        [
            "Exports and auto-export (EXP-02).",
            "",
            "A second paragraph."
        ]
    );
    assert_eq!(section.dialog_only, ["folder", "otherFolder"]);
}

#[test]
fn a_section_needs_a_doc_and_an_order() {
    let undocumented = "register_section! {\n    a: A,\n    order: 1,\n}\n";
    let unordered = "register_section! {\n    /// A.\n    a: A,\n    validate: A::validate,\n}\n";
    let unknown =
        "register_section! {\n    /// A.\n    a: A,\n    order: 1,\n    hidden: true,\n}\n";

    assert!(
        parse_file(undocumented)
            .unwrap_err()
            .contains("no `///` doc")
    );
    assert!(
        parse_file(unordered)
            .unwrap_err()
            .contains("`order: <number>,`")
    );
    assert!(
        parse_file(unknown)
            .unwrap_err()
            .contains("unknown entry `hidden`")
    );
}

#[test]
fn the_macro_definition_and_comments_are_not_registrations() {
    let text = "// register_section! {\nmacro_rules! register_section {\n";

    assert!(parse_file(text).unwrap().is_empty());
}

#[test]
fn keys_become_json_names() {
    assert_eq!(camel("ai"), "ai");
    assert_eq!(camel("speaker_tags"), "speakerTags");
}

#[test]
fn every_section_file_is_read_in_order() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/settings/sections");
    let (modules, sections) = read(&dir).unwrap();

    assert!(modules.contains(&"personal".to_owned()));
    let keys: Vec<&str> = sections
        .iter()
        .map(|section| section.key.as_str())
        .collect();
    assert_eq!(keys.first(), Some(&"ai"));
    assert!(
        sections
            .windows(2)
            .all(|pair| pair[0].order <= pair[1].order)
    );
}
