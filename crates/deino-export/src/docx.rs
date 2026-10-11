//! A Word document (DOCX) that opens in Word, Pages and Google Docs: the
//! title and meta line, `Heading 2` sections, the notes with real headings
//! and lists, ☐/☑ action items, and the transcript as a speaker-labelled
//! table with timestamps. Built with `docx-rs` (MIT).

mod blocks;
mod compress;
mod styles;

use std::io::Cursor;

use docx_rs::{
    Docx, Paragraph, Shading, ShdType, Table, TableCell, TableCellBorder, TableCellBorderPosition,
    TableCellBorders, TableRow, WidthType,
};

use self::blocks::Writer;
use self::styles::{HAIRLINE, MUTED, SUBTLE_FILL};
use crate::labels::{self, Clock, heading};
use crate::{MeetingExport, RenderError, RenderOptions, Section, doc};

/// The transcript table's column widths in twips (time, speaker, text):
/// together the 6.5-inch text width of a Letter page.
const COLUMNS: [usize; 3] = [900, 1_900, 6_560];

/// The meeting as a DOCX file.
pub fn render(export: &MeetingExport, options: &RenderOptions) -> Result<Vec<u8>, RenderError> {
    let mut writer = Writer::new(styles::styled(Docx::new()));
    header(&mut writer, export);
    for section in export.sections(options.sections) {
        write_section(&mut writer, export, section);
    }
    let mut bytes = Cursor::new(Vec::new());
    writer
        .finish()
        .build()
        .pack(&mut bytes)
        .map_err(|error| RenderError::Docx(Box::new(error)))?;
    compress::deflate(bytes.get_ref()).map_err(|error| RenderError::Docx(Box::new(error)))
}

fn header(writer: &mut Writer, export: &MeetingExport) {
    let mut meta = labels::long_date(&export.started_at);
    if let Some(duration) = export.duration {
        meta = format!("{meta} · {}", labels::duration(duration));
    }
    writer.paragraph(text_paragraph(&export.title).style(styles::TITLE));
    writer.paragraph(text_paragraph(&meta).style(styles::META));
}

fn write_section(writer: &mut Writer, export: &MeetingExport, section: Section) {
    let title = match section {
        Section::Attendees => heading::ATTENDEES,
        Section::Notes => heading::NOTES,
        Section::MyNotes => heading::MY_NOTES,
        Section::ActionItems => heading::ACTION_ITEMS,
        Section::Transcript => heading::TRANSCRIPT,
        Section::Insights => return,
    };
    writer.paragraph(text_paragraph(title).style("Heading2"));
    match section {
        Section::Attendees => {
            let names: Vec<&str> = export
                .attendees
                .iter()
                .map(|person| person.name.as_str())
                .collect();
            writer.paragraph(text_paragraph(&names.join(", ")));
        }
        Section::Notes => notes(writer, export.notes.as_deref()),
        Section::MyNotes => notes(writer, export.my_notes.as_deref()),
        Section::ActionItems => action_items(writer, export),
        Section::Transcript => writer.table(transcript(export)),
        Section::Insights => {}
    }
}

fn notes(writer: &mut Writer, markdown: Option<&str>) {
    let blocks = doc::demote(doc::parse(markdown.unwrap_or_default()), 1);
    writer.blocks(&blocks, 0);
}

fn action_items(writer: &mut Writer, export: &MeetingExport) {
    for item in &export.action_items {
        let mark = if item.done { "☑ " } else { "☐ " };
        let details: Vec<String> = item
            .assignee
            .clone()
            .into_iter()
            .chain(
                item.due
                    .map(|date| format!("due {}", labels::short_date(date))),
            )
            .collect();
        let mut paragraph = Paragraph::new()
            .add_run(styles::text(mark))
            .add_run(styles::text(&item.title));
        if !details.is_empty() {
            paragraph = paragraph.add_run(
                styles::text_run()
                    .add_text(format!(" — {}", details.join(", ")))
                    .color(MUTED),
            );
        }
        writer.paragraph(paragraph);
    }
}

fn transcript(export: &MeetingExport) -> Table {
    let clock = Clock::for_transcript(&export.transcript);
    let header = TableRow::new(vec![
        header_cell("Time", COLUMNS[0]),
        header_cell("Speaker", COLUMNS[1]),
        header_cell("Text", COLUMNS[2]),
    ]);
    let rows = export.transcript.iter().map(|line| {
        TableRow::new(vec![
            cell(
                Paragraph::new().add_run(styles::text(clock.format(line.start)).color(MUTED)),
                COLUMNS[0],
            ),
            cell(
                Paragraph::new().add_run(styles::text(&line.speaker).bold()),
                COLUMNS[1],
            ),
            cell(text_paragraph(&line.text), COLUMNS[2]),
        ])
        .cant_split()
    });
    Table::new(std::iter::once(header).chain(rows).collect())
        .clear_all_border()
        .set_grid(COLUMNS.to_vec())
        .width(COLUMNS.iter().sum(), WidthType::Dxa)
}

fn header_cell(text: &str, width: usize) -> TableCell {
    let paragraph = Paragraph::new().add_run(styles::text(text).bold().color(MUTED).size(18));
    cell(paragraph, width).shading(Shading::new().shd_type(ShdType::Clear).fill(SUBTLE_FILL))
}

/// A cell with only a hairline under it.
fn cell(paragraph: Paragraph, width: usize) -> TableCell {
    let hairline = TableCellBorder::new(TableCellBorderPosition::Bottom)
        .color(HAIRLINE)
        .size(4);
    let borders = TableCellBorders::new().clear_all().set(hairline);
    TableCell::new()
        .add_paragraph(paragraph)
        .width(width, WidthType::Dxa)
        .set_borders(borders)
}

fn text_paragraph(text: &str) -> Paragraph {
    Paragraph::new().add_run(styles::text(text))
}

#[cfg(test)]
#[path = "docx_tests.rs"]
mod tests;
