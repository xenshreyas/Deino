use super::*;
use crate::test_support::{fixture_export, fixture_exports};
use crate::{ExportSections, LinkStyle};

fn options(sections: ExportSections, links: LinkStyle) -> RenderOptions {
    RenderOptions { sections, links }
}

#[test]
fn every_fixture_renders_its_shareable_notes() {
    for export in fixture_exports() {
        let name = export
            .id
            .as_str()
            .trim_start_matches("meeting-fixture-")
            .to_owned();
        let markdown = render(&export, &RenderOptions::default());
        insta::assert_snapshot!(format!("markdown_{name}"), markdown);
    }
}

#[test]
fn the_standup_renders_every_section() {
    let export = fixture_export("standup");
    let markdown = render(&export, &options(ExportSections::ALL, LinkStyle::Plain));
    insta::assert_snapshot!("markdown_standup_all", markdown);
}

#[test]
fn obsidian_mode_links_people_and_tags() {
    let mut export = fixture_export("standup");
    export.tags = vec!["Eng standup".into(), "Q4".into()];
    let markdown = render(&export, &options(ExportSections::ALL, LinkStyle::Obsidian));

    insta::assert_snapshot!("markdown_standup_obsidian", markdown);
    assert!(markdown.contains("attendees:\n  - \"[[Shreyas]]\"\n  - \"[[Maya Chen]]\""));
    assert!(markdown.contains("tags:\n  - \"Eng-standup\"\n  - \"Q4\""));
    assert!(markdown.contains("\\\n#Eng-standup #Q4\n"));
    assert!(markdown.contains("- [[Diego Alvarez]]"));
    assert!(markdown.contains("— [[Maya Chen]]") || markdown.contains("— [[Diego Alvarez]]"));
}

#[test]
fn the_owner_of_an_exported_note_is_its_meeting() {
    let export = fixture_export("standup");
    let markdown = render(&export, &RenderOptions::default());
    assert_eq!(owner(&markdown).as_deref(), Some("meeting-fixture-standup"));
}

#[test]
fn unchosen_and_empty_sections_are_left_out() {
    let mut export = fixture_export("standup");
    export.my_notes = Some("   ".into());
    let sections = ExportSections {
        notes: false,
        ..ExportSections::ALL
    };

    let markdown = render(&export, &options(sections, LinkStyle::Plain));

    assert!(!markdown.contains("## Notes"));
    assert!(!markdown.contains("## My notes"));
    assert!(markdown.contains("## Action items"));
    assert!(markdown.contains("## Transcript"));
}

#[test]
fn a_transcript_sidecar_is_linked_instead_of_inlined() {
    let export = fixture_export("standup");
    let note = note(
        &export,
        &options(ExportSections::ALL, LinkStyle::Obsidian),
        Some("2026-10-08 Eng standup – Transcript"),
    );
    let sidecar = transcript_note(&export, &options(ExportSections::ALL, LinkStyle::Obsidian));

    assert!(note.ends_with("## Transcript\n\n[[2026-10-08 Eng standup – Transcript]]\n"));
    assert!(sidecar.contains("part: \"transcript\""));
    assert!(sidecar.contains("# Eng standup – Transcript"));
    assert!(sidecar.contains("`00:00` **Shreyas:** Morning, everyone."));
}

#[test]
fn people_pages_carry_their_email() {
    let page = person_note(&Participant {
        name: "Maya Chen".into(),
        email: Some("maya@deino.dev".into()),
        is_me: false,
    });
    assert_eq!(
        page,
        "---\ntags:\n  - \"person\"\nemail: \"maya@deino.dev\"\n---\n\n# Maya Chen\n"
    );
}
