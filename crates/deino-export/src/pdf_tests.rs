use super::*;
use crate::test_support::{fixture_export, fixture_exports};
use crate::{ExportSections, LinkStyle};

fn all_sections() -> RenderOptions {
    RenderOptions {
        sections: ExportSections::ALL,
        links: LinkStyle::Plain,
    }
}

/// The PDF's text as a reader extracts it, with whitespace collapsed (PDF
/// text extraction splits lines where the layout wrapped them).
fn text_of(pdf: &[u8]) -> String {
    let text = pdf_extract::extract_text_from_mem(pdf).expect("the PDF parses");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn the_pdf_parses_and_holds_the_title_attendees_and_notes() {
    let pdf = render(&fixture_export("standup"), &all_sections()).unwrap();

    assert!(pdf.starts_with(b"%PDF-"));
    let text = text_of(&pdf);
    assert!(text.contains("Eng standup"), "{text}");
    assert!(
        text.contains("Shreyas, Maya Chen, Diego Alvarez, Priya Nair, Tom Becker"),
        "{text}"
    );
    assert!(text.contains("Decisions"));
    assert!(text.contains("Action items"));
    assert!(text.contains("Morning, everyone."));
}

#[test]
fn the_same_meeting_gives_the_same_bytes() {
    let export = fixture_export("one-on-one");
    assert_eq!(
        render(&export, &all_sections()).unwrap(),
        render(&export, &all_sections()).unwrap()
    );
}

#[test]
fn meeting_text_is_never_typeset_as_markup() {
    let mut export = fixture_export("standup");
    export.title = "#set text(fill: red) *bold* $x$ @ref <label> \\".into();
    export.notes = Some("A [link](javascript:alert(1)) and #emph[x]".into());

    let pdf = render(&export, &all_sections()).unwrap();

    let text = text_of(&pdf);
    assert!(
        text.contains("#set text(fill: red) *bold* $x$ @ref <label>"),
        "{text}"
    );
    assert!(text.contains("#emph[x]"), "{text}");
}

#[test]
fn every_fixture_typesets() {
    for export in fixture_exports() {
        let pdf = render(&export, &all_sections());
        assert!(pdf.is_ok(), "{}: {:?}", export.id, pdf.err());
    }
}

/// Writes the standup in every format into `$DEINO_EXPORT_SAMPLES`, for
/// opening in Preview, Pages and Obsidian by hand:
/// `DEINO_EXPORT_SAMPLES=/tmp/x cargo test -p deino-export -- --ignored write_samples`.
#[test]
#[ignore = "writes files for manual inspection"]
fn write_samples() {
    let Ok(dir) = std::env::var("DEINO_EXPORT_SAMPLES") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["standup", "planning"] {
        let export = fixture_export(name);
        for format in crate::ExportFormat::ALL {
            let rendered = crate::render(format, &export, &all_sections()).unwrap();
            std::fs::write(
                dir.join(format!("{name}.{}", format.extension())),
                rendered.bytes,
            )
            .unwrap();
        }
    }
}
