//! [`ExportService`]: one meeting exported, copied or emailed (EXP-01).
//!
//! Every action assembles the meeting, renders it with `deino-export` and
//! hands the result to a port: the save dialog and the vault for files, the
//! clipboard for copies, the URL opener for email drafts. Rendering runs on
//! the blocking pool, because typesetting a long meeting's PDF takes a while.

use std::path::Path;
use std::sync::Arc;

use deino_domain::MeetingId;
use deino_export::naming::NameTemplate;
use deino_export::{
    ExportFormat, ExportSections, MeetingExport, RenderOptions, Rendered, html, mailto, markdown,
    text,
};

use super::{
    Clipboard, ClipboardContent, CopyKind, ExportAssembler, ExportError, ExportPreview,
    PreviewKind, SaveDialog, SaveRequest, SavedExport, UrlOpener, Vault,
};
use crate::settings::DEFAULT_FILE_NAME;

/// What [`ExportService`] is built from.
pub struct ExportServiceDeps {
    /// Reads meetings.
    pub assembler: Arc<ExportAssembler>,
    /// Writes saved files.
    pub vault: Arc<dyn Vault>,
    /// Asks where to save.
    pub save_dialog: Arc<dyn SaveDialog>,
    /// The system clipboard.
    pub clipboard: Arc<dyn Clipboard>,
    /// Opens email drafts.
    pub opener: Arc<dyn UrlOpener>,
}

/// Exports, copies and emails single meetings.
pub struct ExportService {
    pub(super) deps: ExportServiceDeps,
}

impl ExportService {
    /// The service over its ports.
    pub fn new(deps: ExportServiceDeps) -> Self {
        Self { deps }
    }

    /// What exporting the meeting as `format` would write: the text of text
    /// formats, an HTML rendering of DOCX and PDF.
    pub async fn preview(
        &self,
        meeting: &MeetingId,
        format: ExportFormat,
        sections: ExportSections,
    ) -> Result<ExportPreview, ExportError> {
        let export = self.deps.assembler.assemble(meeting).await?;
        let options = RenderOptions {
            sections,
            ..RenderOptions::default()
        };
        let file_name = suggested_name(&export, format);
        let (kind, content) = if format.is_text() {
            let rendered = rendered(format, export, options).await?;
            (
                PreviewKind::Text,
                String::from_utf8_lossy(&rendered.bytes).into_owned(),
            )
        } else {
            (PreviewKind::Html, html::render(&export, &options))
        };
        Ok(ExportPreview {
            format,
            kind,
            content,
            file_name,
        })
    }

    /// Asks where to save the meeting as `format`, then writes it there.
    /// `None` when the user cancelled the dialog.
    pub async fn save(
        &self,
        meeting: &MeetingId,
        format: ExportFormat,
        sections: ExportSections,
    ) -> Result<Option<SavedExport>, ExportError> {
        let export = self.deps.assembler.assemble(meeting).await?;
        let request = SaveRequest {
            file_name: suggested_name(&export, format),
            kind: format.label(),
            extension: format.extension(),
        };
        let Some(path) = self.deps.save_dialog.save_path(request).await else {
            return Ok(None);
        };
        let path = with_extension(&path, format);
        let options = RenderOptions {
            sections,
            ..RenderOptions::default()
        };
        let rendered = rendered(format, export, options).await?;
        self.deps
            .vault
            .write(&path, rendered.bytes)
            .await
            .map_err(ExportError::Write)?;
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Some(SavedExport { file_name }))
    }

    /// Puts the meeting on the clipboard as `kind`.
    pub async fn copy(&self, meeting: &MeetingId, kind: CopyKind) -> Result<(), ExportError> {
        let export = self.deps.assembler.assemble(meeting).await?;
        self.deps
            .clipboard
            .write(clipboard_content(&export, kind))
            .await
    }

    /// Opens an email draft of the notes, addressed to the other attendees.
    pub async fn email(&self, meeting: &MeetingId) -> Result<(), ExportError> {
        let export = self.deps.assembler.assemble(meeting).await?;
        let options = RenderOptions {
            sections: ExportSections::SHARE,
            ..RenderOptions::default()
        };
        self.deps.opener.open(&mailto::url(&export, &options)).await
    }
}

/// What each copy kind puts on the clipboard.
fn clipboard_content(export: &MeetingExport, kind: CopyKind) -> ClipboardContent {
    let share = RenderOptions {
        sections: ExportSections::SHARE,
        ..RenderOptions::default()
    };
    match kind {
        CopyKind::RichText => ClipboardContent {
            text: text::render(export, &share),
            html: Some(html::render(export, &share)),
        },
        CopyKind::Markdown => ClipboardContent {
            text: markdown::render(export, &share),
            html: None,
        },
        CopyKind::Transcript => {
            let options = RenderOptions {
                sections: ExportSections::TRANSCRIPT,
                ..RenderOptions::default()
            };
            ClipboardContent {
                text: text::render(export, &options),
                html: None,
            }
        }
    }
}

/// Renders on the blocking pool (typesetting is CPU-bound).
pub(super) async fn rendered(
    format: ExportFormat,
    export: MeetingExport,
    options: RenderOptions,
) -> Result<Rendered, ExportError> {
    let rendered =
        tokio::task::spawn_blocking(move || deino_export::render(format, &export, &options))
            .await
            .map_err(|_| ExportError::Cancelled)?;
    Ok(rendered?)
}

/// "2026-10-08 Eng standup.pdf".
pub(super) fn suggested_name(export: &MeetingExport, format: ExportFormat) -> String {
    let stem = NameTemplate::parse(DEFAULT_FILE_NAME).map_or_else(
        |_| deino_export::naming::sanitize(&export.title),
        |template| template.stem(export),
    );
    format!("{stem}.{}", format.extension())
}

/// `path`, with the format's extension added if the user removed it.
fn with_extension(path: &Path, format: ExportFormat) -> std::path::PathBuf {
    let matches = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(format.extension()));
    if matches {
        path.to_owned()
    } else {
        let mut name = path.as_os_str().to_owned();
        name.push(".");
        name.push(format.extension());
        name.into()
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
