use super::*;
use crate::test_support::fixture_export;

fn titled(title: &str) -> MeetingExport {
    let mut export = fixture_export("standup");
    export.title = title.to_owned();
    export
}

fn file_name(template: &str, title: &str) -> String {
    NameTemplate::parse(template)
        .unwrap()
        .file_name(&titled(title))
}

#[test]
fn the_default_templates_expand_to_a_dated_note_in_year_and_month_folders() {
    let export = fixture_export("standup");

    let file = NameTemplate::parse("{date} {title}.md")
        .unwrap()
        .file_name(&export);
    let folders = NameTemplate::parse("{yyyy}/{mm}").unwrap().folders(&export);

    assert_eq!(file, "2026-10-08 Eng standup.md");
    assert_eq!(folders, ["2026", "10"]);
}

#[test]
fn every_placeholder_uses_the_local_start_time() {
    let file = file_name("{yyyy}{mm}{dd} {time} {title}", "Sync");
    assert_eq!(file, "20261008 08-31 Sync.md");
}

#[test]
fn slashes_in_titles_never_make_folders() {
    assert_eq!(
        file_name("{title}", "Q3/Q4 review: A\\B"),
        "Q3-Q4 review- A-B.md"
    );
    let folders = NameTemplate::parse("{yyyy}/{title}")
        .unwrap()
        .folders(&titled("../../etc/passwd"));
    assert_eq!(folders, ["2026", "-..-etc-passwd"]);
}

#[test]
fn dots_and_spaces_cannot_hide_or_escape_a_file() {
    assert_eq!(file_name("{title}", "..."), "Untitled.md");
    assert_eq!(file_name("{title}", "  .hidden. "), "hidden.md");
    let folders = NameTemplate::parse("{title}/x")
        .unwrap()
        .folders(&titled(".."));
    assert_eq!(folders, ["Untitled", "x"]);
}

#[test]
fn obsidian_link_syntax_and_wildcards_are_dropped() {
    assert_eq!(
        file_name("{title}", "[[Plan]] #q4 | *draft*?"),
        "Plan q4 draft.md"
    );
    assert_eq!(
        file_name("{title}", "tab\there\nnewline"),
        "tab-here-newline.md"
    );
}

#[test]
fn emoji_and_accents_are_kept() {
    assert_eq!(
        file_name("{date} {title}", "Café ☕ sync 🚀"),
        "2026-10-08 Café ☕ sync 🚀.md"
    );
}

#[test]
fn very_long_titles_are_cut_at_a_character_boundary() {
    let long = "é".repeat(300);
    let stem = NameTemplate::parse("{title}").unwrap().stem(&titled(&long));
    assert_eq!(stem.chars().count(), 100);
    assert!(stem.len() <= 200);

    let emoji = "🚀".repeat(100);
    let stem = NameTemplate::parse("{title}")
        .unwrap()
        .stem(&titled(&emoji));
    assert_eq!(stem.chars().count(), 50);

    let ascii = "a".repeat(500);
    assert_eq!(
        NameTemplate::parse("{title}")
            .unwrap()
            .stem(&titled(&ascii))
            .len(),
        MAX_COMPONENT_CHARS
    );
}

#[test]
fn bad_templates_are_rejected_with_a_reason() {
    assert_eq!(NameTemplate::parse("  .md"), Err(TemplateError::Empty));
    assert_eq!(NameTemplate::parse("{date"), Err(TemplateError::Unclosed));
    assert_eq!(
        NameTemplate::parse("{data} {title}"),
        Err(TemplateError::UnknownField("data".into()))
    );
    assert_eq!(
        TemplateError::UnknownField("data".into()).to_string(),
        "{data} is not a placeholder; use {date}, {time}, {title}, {yyyy}, {mm} or {dd}"
    );
}

#[test]
fn a_template_without_an_extension_still_writes_markdown() {
    assert_eq!(file_name("{title}", "Sync"), "Sync.md");
    assert_eq!(file_name("{title}.MD", "Sync"), "Sync.md");
}
