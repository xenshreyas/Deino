//! The account export's zip archive, written on a blocking thread.
//!
//! ```text
//! deino-export/
//!   meetings/2026/10/2026-10-08 Eng standup.md   (and .json)
//!   audio/2026-10-08 Eng standup.mp4             (when chosen)
//!   meetings.csv  action_items.csv  people.csv
//! ```
//!
//! The archive is written to a hidden temporary file beside the target and
//! renamed into place when complete, so a cancelled or failed export never
//! leaves a broken zip under the name the user chose.

use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use deino_export::naming::NameTemplate;
use deino_export::{MeetingExport, RenderOptions, csv, json, markdown};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::ExportError;
use super::vault::temporary_beside;
use crate::settings::{DEFAULT_FILE_NAME, DEFAULT_SUBFOLDER};

/// The folder everything sits in inside the zip.
const ROOT: &str = "deino-export";

/// One meeting for the archive: its export and, when chosen, its recording.
pub(crate) struct ArchiveEntry {
    pub(crate) export: MeetingExport,
    pub(crate) audio: Option<PathBuf>,
}

/// An archive being written.
pub(crate) struct Archive {
    zip: ZipWriter<File>,
    temporary: PathBuf,
    target: PathBuf,
    names: HashSet<String>,
    /// Each meeting without its notes and transcript, for the CSV tables.
    rows: Vec<MeetingExport>,
}

impl Archive {
    /// Starts an archive that will end up at `target`.
    pub(crate) fn create(target: &Path) -> io::Result<Self> {
        let temporary = temporary_beside(target);
        Ok(Self {
            zip: ZipWriter::new(File::create(&temporary)?),
            temporary,
            target: target.to_owned(),
            names: HashSet::new(),
            rows: Vec::new(),
        })
    }

    /// Adds a meeting's note, JSON and recording.
    pub(crate) fn add(&mut self, entry: ArchiveEntry) -> Result<(), ExportError> {
        let stem = self.unique_stem(&entry.export);
        let note = markdown::render(&entry.export, &RenderOptions::default());
        self.file(&format!("meetings/{stem}.md"), note.as_bytes())?;
        self.file(
            &format!("meetings/{stem}.json"),
            json::render(&entry.export)?.as_bytes(),
        )?;
        if let Some(audio) = entry.audio {
            self.audio(&stem, &audio)?;
        }
        self.rows.push(MeetingExport {
            notes: None,
            my_notes: None,
            transcript: Vec::new(),
            ..entry.export
        });
        Ok(())
    }

    /// Writes the CSV tables, closes the zip and moves it into place; on
    /// failure the unfinished file is deleted.
    pub(crate) fn finish(self) -> Result<(), ExportError> {
        let temporary = self.temporary.clone();
        let finished = self.complete();
        if finished.is_err() {
            // Nothing to do if it is already gone.
            let _removed = std::fs::remove_file(&temporary);
        }
        finished
    }

    fn complete(mut self) -> Result<(), ExportError> {
        self.file("meetings.csv", csv::meetings(&self.rows).as_bytes())?;
        self.file("action_items.csv", csv::action_items(&self.rows).as_bytes())?;
        self.file("insights.csv", csv::insights(&self.rows).as_bytes())?;
        self.file("people.csv", csv::people(&self.rows).as_bytes())?;
        self.zip
            .finish()
            .map_err(|error| ExportError::Write(io::Error::other(error)))?;
        std::fs::rename(&self.temporary, &self.target).map_err(ExportError::Write)
    }

    /// Throws the unfinished archive away.
    pub(crate) fn discard(self) {
        drop(self.zip);
        // Nothing to do if it is already gone.
        let _removed = std::fs::remove_file(&self.temporary);
    }

    /// `yyyy/mm/{date} {title}`, numbered when two meetings share it.
    fn unique_stem(&mut self, export: &MeetingExport) -> String {
        let folders = NameTemplate::parse(DEFAULT_SUBFOLDER)
            .map(|template| template.folders(export))
            .unwrap_or_default();
        let name = NameTemplate::parse(DEFAULT_FILE_NAME)
            .map_or_else(|_| export.id.to_string(), |template| template.stem(export));
        let base = folders
            .into_iter()
            .chain([name])
            .collect::<Vec<_>>()
            .join("/");
        let mut stem = base.clone();
        let mut suffix = 1;
        while !self.names.insert(stem.clone()) {
            suffix += 1;
            stem = format!("{base} ({suffix})");
        }
        stem
    }

    fn file(&mut self, name: &str, bytes: &[u8]) -> Result<(), ExportError> {
        self.start(name, CompressionMethod::Deflated)?;
        self.zip.write_all(bytes).map_err(ExportError::Write)
    }

    /// Recordings are already compressed, so they are stored as they are.
    fn audio(&mut self, stem: &str, path: &Path) -> Result<(), ExportError> {
        let name = Path::new(stem)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut file = File::open(path).map_err(ExportError::Audio)?;
        self.start(
            &format!("audio/{name}.{extension}"),
            CompressionMethod::Stored,
        )?;
        io::copy(&mut file, &mut self.zip).map_err(ExportError::Audio)?;
        Ok(())
    }

    fn start(&mut self, name: &str, method: CompressionMethod) -> Result<(), ExportError> {
        let options = SimpleFileOptions::default()
            .compression_method(method)
            .large_file(true);
        self.zip
            .start_file(format!("{ROOT}/{name}"), options)
            .map_err(|error| ExportError::Write(io::Error::other(error)))
    }
}
