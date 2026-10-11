//! `deino export <id> --format md|txt|json|pdf|docx|srt|vtt`: one meeting
//! through the app's own exporters (`deino-core::export` assembles it,
//! `deino-export` renders it), so the file matches the app's Share menu.

use std::path::{Path, PathBuf};

use deino_core::export::ExportAssembler;
use deino_core::settings::DEFAULT_FILE_NAME;
use deino_export::naming::NameTemplate;
use deino_export::{ExportFormat, ExportSections, LinkStyle, RenderOptions, render};
use serde::Serialize;

use crate::CliError;
use crate::request::meeting_id;

/// The formats on the command line.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum FormatArg {
    /// Markdown with YAML frontmatter.
    Md,
    /// Plain text.
    Txt,
    /// The whole meeting as JSON (`deino.meeting` v1).
    Json,
    /// A PDF document.
    Pdf,
    /// A Word document.
    Docx,
    /// The transcript as SubRip subtitles.
    Srt,
    /// The transcript as WebVTT subtitles.
    Vtt,
}

impl FormatArg {
    const fn format(self) -> ExportFormat {
        match self {
            Self::Md => ExportFormat::Markdown,
            Self::Txt => ExportFormat::Text,
            Self::Json => ExportFormat::Json,
            Self::Pdf => ExportFormat::Pdf,
            Self::Docx => ExportFormat::Docx,
            Self::Srt => ExportFormat::Srt,
            Self::Vtt => ExportFormat::Vtt,
        }
    }
}

/// `deino export`.
#[derive(Debug, clap::Args)]
pub struct ExportArgs {
    /// The meeting's ID.
    pub id: String,
    /// The file format.
    #[arg(long, short, value_enum, default_value = "md")]
    pub format: FormatArg,
    /// Where to write the file. Text formats print to stdout without it;
    /// PDF and DOCX are written to "<date> <title>.<ext>" here.
    #[arg(long, short)]
    pub out: Option<PathBuf>,
    /// Include the transcript (always in JSON and subtitles).
    #[arg(long)]
    pub transcript: bool,
}

/// What `--json` prints after writing a file.
#[derive(Debug, Serialize)]
struct Written {
    path: String,
    format: ExportFormat,
    bytes: usize,
}

impl ExportArgs {
    /// Renders the meeting and prints it or writes it; returns what to print.
    pub async fn run(self, exports: &ExportAssembler, json: bool) -> Result<String, CliError> {
        let export = exports.assemble(&meeting_id(&self.id)?).await?;
        let format = self.format.format();
        let sections = if self.transcript {
            ExportSections::ALL
        } else {
            ExportSections::default()
        };
        let options = RenderOptions {
            sections,
            links: LinkStyle::Plain,
        };
        let rendered = render(format, &export, &options)?;
        if let (None, Some(text)) = (&self.out, rendered.text()) {
            return Ok(text.to_owned());
        }
        let path = match self.out {
            Some(path) => path,
            None => default_path(&export, format)?,
        };
        write(&path, &rendered.bytes)?;
        let written = Written {
            path: path.display().to_string(),
            format,
            bytes: rendered.bytes.len(),
        };
        if json {
            return Ok(format!("{}\n", serde_json::to_string_pretty(&written)?));
        }
        Ok(format!("Wrote {}\n", written.path))
    }
}

/// `<date> <title>.<ext>` in the current folder, as the app names files.
fn default_path(
    export: &deino_export::MeetingExport,
    format: ExportFormat,
) -> Result<PathBuf, CliError> {
    let template = NameTemplate::parse(DEFAULT_FILE_NAME)
        .map_err(|error| CliError::Invalid(error.to_string()))?;
    Ok(PathBuf::from(format!(
        "{}.{}",
        template.stem(export),
        format.extension()
    )))
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    std::fs::write(path, bytes).map_err(|source| CliError::Write {
        path: path.to_path_buf(),
        source,
    })
}
