//! A PDF typeset with Typst (ADR 0025): the template in `template.typ`, the
//! meeting as JSON data, Inter bundled in the binary.
//!
//! Typst runs in-process and is deterministic: the same meeting gives the
//! same bytes, on any machine, with no webview and no system fonts, so the
//! pipeline can write PDFs headlessly and tests read them back.

mod model;
mod world;

use typst::foundations::Smart;
use typst_pdf::PdfOptions;

use self::model::PrintModel;
use self::world::PrintWorld;
use crate::{MeetingExport, RenderError, RenderOptions};

/// The PDF's `/Creator`.
const CREATOR: &str = "Deino";

/// The meeting as a PDF.
pub fn render(export: &MeetingExport, options: &RenderOptions) -> Result<Vec<u8>, RenderError> {
    let model = PrintModel::new(export, *options);
    let data = serde_json::to_vec(&model).map_err(RenderError::Json)?;
    let world = PrintWorld::new(data).ok_or(RenderError::Typeset(1))?;
    let document = typst::compile::<typst_layout::PagedDocument>(&world)
        .output
        .map_err(|errors| RenderError::Typeset(errors.len()))?;
    let pdf_options = PdfOptions {
        ident: Smart::Custom(export.id.to_string()),
        creator: Smart::Custom(Some(CREATOR.to_owned())),
        ..PdfOptions::default()
    };
    typst_pdf::pdf(&document, &pdf_options).map_err(|errors| RenderError::Pdf(errors.len()))
}

#[cfg(test)]
#[path = "pdf_tests.rs"]
mod tests;
