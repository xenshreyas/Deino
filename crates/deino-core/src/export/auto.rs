//! Auto-export (EXP-02): every processed meeting written as a Markdown note
//! into the folder the user chose, an Obsidian vault for example.
//!
//! The note's path comes from the settings' templates
//! (`<folder>/{yyyy}/{mm}/{date} {title}.md`). Auto-export only ever
//! overwrites its own notes: a file is Deino's when its frontmatter carries
//! the meeting's `deino-id`. A file of the same name that is not this
//! meeting's is left alone and the note takes the next free name
//! (`… (2).md`). In Obsidian mode, attendees get a page under `People/`
//! when they have none, so their `[[wiki links]]` resolve; existing pages
//! are never touched.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::MeetingId;
use deino_export::markdown::{self, owner, page_name};
use deino_export::naming::NameTemplate;
use deino_export::{LinkStyle, MeetingExport, RenderOptions};

use super::{ExportAssembler, ExportError, FolderDialog, Vault};
use crate::settings::{ExportSettings, Settings, SettingsService};

/// The folder people pages go in, inside the export folder.
pub const PEOPLE_FOLDER: &str = "People";
/// How many `(n)` suffixes are tried before giving up on a name.
const MAX_SUFFIX: u32 = 50;

/// Why auto-export did nothing; the text is shown on the meeting page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoExportSkip {
    /// Auto-export is off.
    Disabled,
    /// The note exists and overwriting is off.
    AlreadyExported,
}

impl AutoExportSkip {
    /// A user-safe sentence.
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Disabled => "Auto-export is off.",
            Self::AlreadyExported => "The note was exported before; overwriting is off.",
        }
    }
}

/// What auto-export did for one meeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoExportOutcome {
    /// Notes were written: the meeting's, any sidecar and new people pages.
    Exported {
        /// How many files were written.
        files: usize,
    },
    /// Nothing was written.
    Skipped(AutoExportSkip),
}

/// Exports one meeting after processing; the `auto_export` stage's port.
#[async_trait]
pub trait AutoExporter: Send + Sync {
    /// Exports `meeting` if the settings say so.
    async fn export(&self, meeting: &MeetingId) -> Result<AutoExportOutcome, ExportError>;
}

/// [`AutoExporter`] over the settings, the repositories and the vault.
pub struct AutoExportService {
    assembler: Arc<ExportAssembler>,
    settings: Arc<SettingsService>,
    vault: Arc<dyn Vault>,
}

/// Where a meeting's notes go, from the settings.
struct Plan {
    folder: PathBuf,
    stem: String,
    options: RenderOptions,
}

impl AutoExportService {
    /// The service over its ports.
    pub fn new(
        assembler: Arc<ExportAssembler>,
        settings: Arc<SettingsService>,
        vault: Arc<dyn Vault>,
    ) -> Self {
        Self {
            assembler,
            settings,
            vault,
        }
    }

    /// Asks for the export folder and saves it; `None` when cancelled.
    pub async fn choose_folder(
        &self,
        dialog: &dyn FolderDialog,
    ) -> Result<Option<Settings>, ExportError> {
        let Some(folder) = dialog.pick_folder().await else {
            return Ok(None);
        };
        let folder = folder.to_string_lossy().into_owned();
        let settings = self
            .settings
            .update(|settings| settings.export.folder = Some(folder))
            .await?;
        Ok(Some(settings))
    }

    async fn run(
        &self,
        meeting: &MeetingId,
        settings: &ExportSettings,
    ) -> Result<AutoExportOutcome, ExportError> {
        let root = PathBuf::from(settings.folder.as_deref().ok_or(ExportError::NoFolder)?);
        if !self.vault.is_dir(&root).await {
            return Err(ExportError::FolderUnavailable(
                io::ErrorKind::NotFound.into(),
            ));
        }
        let export = self.assembler.assemble(meeting).await?;
        let plan = plan(&root, settings, &export);
        let (note, owned) = self.claim(&plan.folder, &plan.stem, meeting).await?;
        if owned && !settings.overwrite {
            return Ok(AutoExportOutcome::Skipped(AutoExportSkip::AlreadyExported));
        }
        let mut files = 0;
        let mut options = plan.options;
        let mut sidecar_link = None;
        if settings.transcript_sidecar && !export.transcript.is_empty() {
            sidecar_link = Some(self.write_sidecar(&plan, &note, &export).await?);
            files += 1;
            options.sections.transcript = false;
        }
        let body = markdown::note(&export, &options, sidecar_link.as_deref());
        self.write(&path_of(&plan.folder, &note), body).await?;
        files += 1;
        if settings.obsidian {
            files += self.write_people(&root, &export).await?;
        }
        Ok(AutoExportOutcome::Exported { files })
    }

    /// The first name from `stem` that is free or this meeting's own note,
    /// and whether it is the meeting's own.
    async fn claim(
        &self,
        folder: &Path,
        stem: &str,
        meeting: &MeetingId,
    ) -> Result<(String, bool), ExportError> {
        for suffix in 1..=MAX_SUFFIX {
            let name = if suffix == 1 {
                stem.to_owned()
            } else {
                format!("{stem} ({suffix})")
            };
            let head = self
                .vault
                .head(&path_of(folder, &name))
                .await
                .map_err(ExportError::FolderUnavailable)?;
            match head {
                None => return Ok((name, false)),
                Some(head) if owner(&head).as_deref() == Some(meeting.as_str()) => {
                    return Ok((name, true));
                }
                Some(_) => {}
            }
        }
        Err(ExportError::Write(io::Error::other(
            "every name for the note is taken",
        )))
    }

    /// Writes the transcript note beside `note` (under a free name if a file
    /// that is not this meeting's has it); returns the sidecar's name.
    async fn write_sidecar(
        &self,
        plan: &Plan,
        note: &str,
        export: &MeetingExport,
    ) -> Result<String, ExportError> {
        let sidecar = format!("{note} – Transcript");
        let (stem, _) = self.claim(&plan.folder, &sidecar, &export.id).await?;
        let body = markdown::transcript_note(export, &plan.options);
        self.write(&path_of(&plan.folder, &stem), body).await?;
        Ok(stem)
    }

    /// Writes a page for each attendee who has none; returns how many.
    async fn write_people(
        &self,
        root: &Path,
        export: &MeetingExport,
    ) -> Result<usize, ExportError> {
        let folder = root.join(PEOPLE_FOLDER);
        let mut written = 0;
        for person in &export.attendees {
            let path = path_of(&folder, &page_name(&person.name));
            if self
                .vault
                .head(&path)
                .await
                .map_err(ExportError::FolderUnavailable)?
                .is_none()
            {
                self.write(&path, markdown::person_note(person)).await?;
                written += 1;
            }
        }
        Ok(written)
    }

    async fn write(&self, path: &Path, text: String) -> Result<(), ExportError> {
        self.vault
            .write(path, text.into_bytes())
            .await
            .map_err(ExportError::Write)
    }
}

#[async_trait]
impl AutoExporter for AutoExportService {
    async fn export(&self, meeting: &MeetingId) -> Result<AutoExportOutcome, ExportError> {
        let settings = self.settings.get().await.export;
        if !settings.auto_export {
            return Ok(AutoExportOutcome::Skipped(AutoExportSkip::Disabled));
        }
        self.run(meeting, &settings).await
    }
}

/// The folder and file stem the settings give `export`, and how to render it.
fn plan(root: &Path, settings: &ExportSettings, export: &MeetingExport) -> Plan {
    let folders = NameTemplate::parse(&settings.subfolder)
        .map(|template| template.folders(export))
        .unwrap_or_default();
    let stem = NameTemplate::parse(&settings.file_name).map_or_else(
        |_| deino_export::naming::sanitize(&export.title),
        |template| template.stem(export),
    );
    let links = if settings.obsidian {
        LinkStyle::Obsidian
    } else {
        LinkStyle::Plain
    };
    Plan {
        folder: folders
            .iter()
            .fold(root.to_owned(), |path, folder| path.join(folder)),
        stem,
        options: RenderOptions {
            sections: settings.sections,
            links,
        },
    }
}

fn path_of(folder: &Path, stem: &str) -> PathBuf {
    folder.join(format!("{stem}.md"))
}

#[cfg(test)]
#[path = "auto_tests.rs"]
mod tests;
