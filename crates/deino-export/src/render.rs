//! [`render`]: one meeting in one format.

use crate::{
    ExportFormat, MeetingExport, RenderError, RenderOptions, docx, json, markdown, pdf, subtitles,
    text,
};

/// A rendered file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    /// What it was rendered as.
    pub format: ExportFormat,
    /// The file's contents.
    pub bytes: Vec<u8>,
}

impl Rendered {
    /// The contents as text, for text formats (`None` for DOCX and PDF).
    pub fn text(&self) -> Option<&str> {
        if self.format.is_text() {
            std::str::from_utf8(&self.bytes).ok()
        } else {
            None
        }
    }
}

/// `export` rendered as `format`. Formats with a fixed content (JSON,
/// subtitles) ignore the chosen sections.
pub fn render(
    format: ExportFormat,
    export: &MeetingExport,
    options: &RenderOptions,
) -> Result<Rendered, RenderError> {
    let bytes = match format {
        ExportFormat::Markdown => markdown::render(export, options).into_bytes(),
        ExportFormat::Text => text::render(export, options).into_bytes(),
        ExportFormat::Json => json::render(export)?.into_bytes(),
        ExportFormat::Srt => subtitles::srt(export).into_bytes(),
        ExportFormat::Vtt => subtitles::vtt(export).into_bytes(),
        ExportFormat::Docx => docx::render(export, options)?,
        ExportFormat::Pdf => pdf::render(export, options)?,
    };
    Ok(Rendered { format, bytes })
}
