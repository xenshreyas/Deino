use std::io::{Cursor, Read};

use super::*;
use crate::test_support::fixture_export;
use crate::{ExportSections, LinkStyle};

/// The named part of a DOCX package, as text.
fn part(docx: &[u8], name: &str) -> String {
    let mut archive = zip::ZipArchive::new(Cursor::new(docx)).expect("a zip package");
    let mut file = archive.by_name(name).expect("the part exists");
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    text
}

fn all_sections() -> RenderOptions {
    RenderOptions {
        sections: ExportSections::ALL,
        links: LinkStyle::Plain,
    }
}

#[test]
fn the_package_has_the_parts_word_and_pages_need() {
    let docx = render(&fixture_export("standup"), &all_sections()).unwrap();

    let types = part(&docx, "[Content_Types].xml");
    assert!(types.contains("wordprocessingml.document.main+xml"));
    for name in [
        "word/document.xml",
        "word/styles.xml",
        "word/numbering.xml",
        "_rels/.rels",
    ] {
        assert!(!part(&docx, name).is_empty(), "{name}");
    }
}

#[test]
fn the_document_holds_the_title_attendees_notes_and_transcript() {
    let docx = render(&fixture_export("standup"), &all_sections()).unwrap();
    let document = part(&docx, "word/document.xml");

    assert!(document.contains(">Eng standup<"));
    assert!(document.contains("Shreyas, Maya Chen, Diego Alvarez, Priya Nair, Tom Becker"));
    assert!(document.contains("w:val=\"Heading2\""));
    assert!(document.contains(">Decisions<"));
    assert!(document.contains("☐ ") && document.contains("☑ "));
    assert!(document.contains("<w:tbl>"));
    assert!(document.contains(">Morning, everyone. Let&apos;s keep it quick today, I&apos;ve got a hard stop at quarter to.<")
        || document.contains(">Morning, everyone. Let's keep it quick today, I've got a hard stop at quarter to.<"));
}

#[test]
fn markup_in_meeting_text_stays_text() {
    let mut export = fixture_export("standup");
    export.title = "R&D <review>".into();

    let docx = render(&export, &RenderOptions::default()).unwrap();

    assert!(part(&docx, "word/document.xml").contains("R&amp;D &lt;review&gt;"));
}

#[test]
fn every_fixture_renders() {
    for export in crate::test_support::fixture_exports() {
        assert!(render(&export, &all_sections()).is_ok(), "{}", export.id);
    }
}
