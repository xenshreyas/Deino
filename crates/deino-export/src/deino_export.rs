//! Meeting exports as pure renderers (EXP-01, EXP-02, ADR 0025).
//!
//! Everything here is a function of a [`MeetingExport`] (the typed snapshot
//! of one meeting that `deino-core`'s export service assembles from the
//! repositories) and [`RenderOptions`] (which [`Section`]s to include and how
//! to write links). Nothing reads the database, the clock, the time zone or
//! the file system, so every format is golden-tested.
//!
//! - [`render`]: one meeting in an [`ExportFormat`]: Markdown with YAML
//!   frontmatter ([`markdown`]), plain text ([`text`]), the full object as
//!   JSON ([`json`]), SRT and VTT subtitles ([`subtitles`]), DOCX ([`docx`])
//!   and PDF ([`pdf`], typeset with typst and the bundled Inter);
//! - [`html`]: the rich-text clipboard flavour and the export preview;
//! - [`doc`]: notes Markdown parsed once into typed [`Block`]s, which every
//!   rich format walks;
//! - [`naming`]: file and folder names from user templates
//!   (`{date} {title}.md`, `{yyyy}/{mm}`), safe on every file system;
//! - [`csv`]: the full account export's tables, and one insight's answers
//!   across meetings ([`insights`], AI-06);
//! - [`mailto`]: an email draft URL with the notes in its body.
//!
//! Notes arrive as Markdown. When the notepad stores TipTap JSON (NOT-01),
//! `deino-core` converts it through `deino-doc` before it reaches here.

pub mod csv;
pub mod doc;
pub mod docx;
mod error;
pub mod html;
pub mod insights;
pub mod json;
mod labels;
pub mod mailto;
pub mod markdown;
mod model;
pub mod naming;
mod options;
pub mod pdf;
mod render;
pub mod subtitles;
#[cfg(test)]
mod test_support;
pub mod text;

pub use doc::{Block, Inline, ListItem};
pub use error::RenderError;
pub use insights::{
    ExportField, InsightAnswerExport, InsightExport, InsightExportValue, InsightRowExport,
    InsightTableExport,
};
pub use model::{ActionItemExport, MeetingExport, Participant, Utterance};
pub use options::{ExportFormat, ExportSections, LinkStyle, RenderOptions, Section};
pub use render::{Rendered, render};
