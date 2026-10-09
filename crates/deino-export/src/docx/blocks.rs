//! Notes [`Block`]s as Word paragraphs: headings by style, lists on the
//! document's numberings, tasks as ☐/☑ items, inline formatting as runs.

use docx_rs::{
    Docx, Hyperlink, HyperlinkType, IndentLevel, LevelOverride, Numbering, NumberingId, Paragraph,
    Run, RunFonts, SpecialIndentType,
};

use super::styles::{self, MUTED};
use crate::doc::{Block, Inline, ListItem};

/// Builds the document body, handing out a fresh numbering to each
/// numbered list so every one starts at 1.
pub(crate) struct Writer {
    docx: Docx,
    next_ordered: usize,
}

/// The formatting nested inlines inherit: a set of [`Format`] flags.
#[derive(Clone, Copy, Default)]
struct Format(u8);

impl Format {
    const BOLD: u8 = 1;
    const ITALIC: u8 = 1 << 1;
    const STRIKE: u8 = 1 << 2;
    const CODE: u8 = 1 << 3;

    fn with(self, flag: u8) -> Self {
        Self(self.0 | flag)
    }

    fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
}

impl Writer {
    pub(crate) fn new(docx: Docx) -> Self {
        Self {
            docx,
            next_ordered: styles::FIRST_ORDERED_ID,
        }
    }

    pub(crate) fn paragraph(&mut self, paragraph: Paragraph) {
        let docx = std::mem::take(&mut self.docx);
        self.docx = docx.add_paragraph(paragraph);
    }

    pub(crate) fn table(&mut self, table: docx_rs::Table) {
        let docx = std::mem::take(&mut self.docx);
        self.docx = docx.add_table(table);
    }

    pub(crate) fn finish(self) -> Docx {
        self.docx
    }

    /// Writes `blocks` at list depth `depth`.
    pub(crate) fn blocks(&mut self, blocks: &[Block], depth: usize) {
        for block in blocks {
            self.block(block, depth);
        }
    }

    fn block(&mut self, block: &Block, depth: usize) {
        match block {
            Block::Heading { level, content } => {
                let style = format!("Heading{}", level.clamp(&1, &6));
                self.paragraph(runs(Paragraph::new().style(&style), content));
            }
            Block::Paragraph { content } => self.paragraph(runs(Paragraph::new(), content)),
            Block::List { ordered, items } => self.list(*ordered, items, depth),
            Block::Quote { blocks } => self.quote(blocks),
            Block::Code { text } => self.code(text),
            Block::Rule => {
                self.paragraph(Paragraph::new().add_run(Run::new().add_text("———").color(MUTED)))
            }
        }
    }

    fn list(&mut self, ordered: bool, items: &[ListItem], depth: usize) {
        let numbering = if ordered {
            self.ordered_numbering()
        } else {
            styles::BULLETS
        };
        let level = depth.min(styles::LIST_LEVELS - 1);
        for item in items {
            let (head, rest) = split_item(item);
            let paragraph = match item.checked {
                Some(done) => task(done, level),
                None => {
                    Paragraph::new().numbering(NumberingId::new(numbering), IndentLevel::new(level))
                }
            };
            self.paragraph(runs(paragraph, head));
            self.blocks(rest, depth + 1);
        }
    }

    fn ordered_numbering(&mut self) -> usize {
        let id = self.next_ordered;
        self.next_ordered += 1;
        let numbering = Numbering::new(id, styles::ORDERED_ABSTRACT)
            .add_override(LevelOverride::new(0).start(1));
        let docx = std::mem::take(&mut self.docx);
        self.docx = docx.add_numbering(numbering);
        id
    }

    fn quote(&mut self, blocks: &[Block]) {
        for block in blocks {
            let content = inline_text(block);
            let paragraph = Paragraph::new()
                .indent(Some(styles::LIST_INDENT), None, None, None)
                .color(MUTED)
                .italic();
            self.paragraph(runs(paragraph, &content));
        }
    }

    fn code(&mut self, text: &str) {
        let mono = RunFonts::new().ascii("Menlo").hi_ansi("Menlo");
        for line in text.lines() {
            self.paragraph(
                Paragraph::new().add_run(Run::new().add_text(line).fonts(mono.clone()).size(19)),
            );
        }
    }
}

/// An item's first paragraph (its text) and the blocks after it.
fn split_item(item: &ListItem) -> (&[Inline], &[Block]) {
    match item.blocks.as_slice() {
        [Block::Paragraph { content }, rest @ ..] => (content, rest),
        blocks => (&[], blocks),
    }
}

/// A task item: indented like a list item, led by a ☐ or ☑ box.
fn task(done: bool, level: usize) -> Paragraph {
    let indent = styles::LIST_INDENT * (i32::try_from(level).unwrap_or(0) + 1);
    let mark = if done { "☑ " } else { "☐ " };
    Paragraph::new()
        .indent(
            Some(indent),
            Some(SpecialIndentType::Hanging(styles::LIST_INDENT)),
            None,
            None,
        )
        .add_run(Run::new().add_text(mark))
}

/// A nested list or code inside a quote, flattened to its text.
fn inline_text(block: &Block) -> Vec<Inline> {
    match block {
        Block::Code { text } => vec![Inline::text(text.clone())],
        Block::List { items, .. } => items
            .iter()
            .flat_map(|item| item.blocks.iter().flat_map(inline_text))
            .collect(),
        Block::Paragraph { content } | Block::Heading { content, .. } => content.clone(),
        Block::Quote { blocks } => blocks.iter().flat_map(inline_text).collect(),
        Block::Rule => Vec::new(),
    }
}

/// `paragraph` with `content` appended as runs.
pub(crate) fn runs(paragraph: Paragraph, content: &[Inline]) -> Paragraph {
    content.iter().fold(paragraph, |paragraph, inline| {
        add(paragraph, inline, Format::default())
    })
}

fn add(paragraph: Paragraph, inline: &Inline, format: Format) -> Paragraph {
    let nested = |paragraph: Paragraph, content: &[Inline], format: Format| {
        content.iter().fold(paragraph, |paragraph, inline| {
            add(paragraph, inline, format)
        })
    };
    match inline {
        Inline::Text { text } => paragraph.add_run(run(text, format)),
        Inline::Code { text } => paragraph.add_run(run(text, format.with(Format::CODE))),
        Inline::Strong { content } => nested(paragraph, content, format.with(Format::BOLD)),
        Inline::Emphasis { content } => nested(paragraph, content, format.with(Format::ITALIC)),
        Inline::Strike { content } => nested(paragraph, content, format.with(Format::STRIKE)),
        Inline::Link { href, content } => {
            let text = crate::doc::plain(content);
            let link = Hyperlink::new(href, HyperlinkType::External)
                .add_run(run(&text, format).color(styles::INK).underline("single"));
            paragraph.add_hyperlink(link)
        }
        Inline::Break => paragraph.add_run(Run::new().add_break(docx_rs::BreakType::TextWrapping)),
    }
}

fn run(text: &str, format: Format) -> Run {
    let mut run = Run::new().add_text(text);
    if format.has(Format::BOLD) {
        run = run.bold();
    }
    if format.has(Format::ITALIC) {
        run = run.italic();
    }
    if format.has(Format::STRIKE) {
        run = run.strike();
    }
    if format.has(Format::CODE) {
        run = run
            .fonts(RunFonts::new().ascii("Menlo").hi_ansi("Menlo"))
            .size(19);
    }
    run
}
