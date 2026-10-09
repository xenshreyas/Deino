//! The Word document's look: page, fonts, paragraph styles and the two list
//! numberings, on the design tokens' light colours (ui.md §2.2).

use docx_rs::{
    AbstractNumbering, Docx, Level, LevelJc, LevelText, LineSpacing, NumberFormat, PageMargin,
    RunFonts, SpecialIndentType, Start, Style, StyleType,
};

/// Every Mac has it, Pages and Word render it well, and Windows Word
/// substitutes Arial; Inter is not installed on most machines.
const FONT: &str = "Helvetica Neue";
/// `--fg`, `--fg-muted` and `--border` (light theme), as Word hex colours.
pub(crate) const INK: &str = "1F1F1C";
pub(crate) const MUTED: &str = "65655F";
pub(crate) const HAIRLINE: &str = "E6E6E1";
/// `--bg-subtle`, the transcript table's header fill.
pub(crate) const SUBTLE_FILL: &str = "F5F5F2";

/// The body text size, in half-points (10.5 pt).
const BODY_SIZE: usize = 21;
/// US Letter in twentieths of a point, with one-inch margins.
const PAGE: (u32, u32) = (12_240, 15_840);
const MARGIN: i32 = 1_440;

/// Style IDs used by the document.
pub(crate) const TITLE: &str = "Title";
pub(crate) const META: &str = "Meta";

/// The numbering of bulleted lists (`numId` 1) and the abstract numbering
/// of numbered lists; each numbered list gets its own `numId` from
/// [`FIRST_ORDERED_ID`] so it restarts at 1.
pub(crate) const BULLETS: usize = 1;
pub(crate) const ORDERED_ABSTRACT: usize = 2;
pub(crate) const FIRST_ORDERED_ID: usize = 2;
/// How many nesting levels the list numberings define.
pub(crate) const LIST_LEVELS: usize = 4;
/// Indentation per list level, in twips.
pub(crate) const LIST_INDENT: i32 = 360;

/// `docx` with the page set up, the styles defined and the numberings added.
pub(crate) fn styled(docx: Docx) -> Docx {
    let fonts = RunFonts::new()
        .ascii(FONT)
        .hi_ansi(FONT)
        .cs(FONT)
        .east_asia(FONT);
    let docx = docx
        .page_size(PAGE.0, PAGE.1)
        .page_margin(
            PageMargin::new()
                .top(MARGIN)
                .bottom(MARGIN)
                .left(MARGIN)
                .right(MARGIN),
        )
        .default_fonts(fonts)
        .default_size(BODY_SIZE)
        .default_line_spacing(LineSpacing::new().after(120).line(276))
        .add_style(paragraph_style(TITLE, "Title", 44).bold())
        .add_style(paragraph_style(META, "Meta", 19).color(MUTED));
    let docx = (1..=6).fold(docx, |docx, level| docx.add_style(heading_style(level)));
    docx.add_abstract_numbering(numbering(BULLETS, "bullet", |_| "•".to_owned()))
        .add_abstract_numbering(numbering(ORDERED_ABSTRACT, "decimal", |level| {
            format!("%{}.", level + 1)
        }))
        .add_numbering(docx_rs::Numbering::new(BULLETS, BULLETS))
}

fn paragraph_style(id: &str, name: &str, size: usize) -> Style {
    Style::new(id, StyleType::Paragraph)
        .name(name)
        .size(size)
        .color(INK)
}

/// "Heading 1" … "Heading 6": 600-weight, sizes stepping down from 18 pt;
/// levels 3 and below in `--fg-muted`, like note headings in the app.
fn heading_style(level: usize) -> Style {
    let size = [36, 28, 24, 22, 21, 21]
        .get(level - 1)
        .copied()
        .unwrap_or(BODY_SIZE);
    let style = Style::new(format!("Heading{level}"), StyleType::Paragraph)
        .name(format!("Heading {level}"))
        .size(size)
        .bold()
        .outline_lvl(level - 1)
        .line_spacing(LineSpacing::new().before(240).after(80));
    if level >= 4 {
        style.color(MUTED)
    } else {
        style.color(INK)
    }
}

fn numbering(id: usize, format: &str, text: impl Fn(usize) -> String) -> AbstractNumbering {
    (0..LIST_LEVELS).fold(AbstractNumbering::new(id), |numbering, level| {
        let indent = LIST_INDENT * (i32::try_from(level).unwrap_or(0) + 1);
        numbering.add_level(
            Level::new(
                level,
                Start::new(1),
                NumberFormat::new(format),
                LevelText::new(text(level)),
                LevelJc::new("left"),
            )
            .indent(
                Some(indent),
                Some(SpecialIndentType::Hanging(LIST_INDENT)),
                None,
                None,
            ),
        )
    })
}
