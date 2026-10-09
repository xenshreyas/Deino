//! [`LegacyTemplateFolder`]: the user's custom JSON templates from before
//! NOT-04 (`<app_data_dir>/templates/*.json`), moved into the database once.
//!
//! Each file that parses becomes one of the user's templates; it is then
//! moved into `imported/` inside the folder, so the next start does not
//! import it again and nothing the user wrote is deleted. A file that does
//! not parse stays where it is and is logged by name only.

use std::path::{Path, PathBuf};

use super::{TemplateContent, TemplateError, import_document};

/// Where imported files are moved, inside the legacy folder.
pub const IMPORTED_FOLDER: &str = "imported";

/// The pre-NOT-04 custom templates folder.
#[derive(Clone, Debug)]
pub struct LegacyTemplateFolder {
    dir: PathBuf,
}

/// One legacy file that parsed.
#[derive(Clone, Debug)]
pub struct LegacyTemplateFile {
    /// Where it is.
    pub path: PathBuf,
    /// What it says, not yet validated.
    pub content: TemplateContent,
}

impl LegacyTemplateFolder {
    /// The folder at `dir` (it need not exist).
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Every `*.json` file directly in the folder that parses as a template,
    /// by file name. A missing folder has none.
    pub fn templates(&self) -> Result<Vec<LegacyTemplateFile>, TemplateError> {
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(TemplateError::LegacyFolder(error)),
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "json"))
            .collect();
        paths.sort();
        Ok(paths.into_iter().filter_map(|path| parse(&path)).collect())
    }

    /// Moves an imported file into `imported/`.
    pub fn archive(&self, file: &LegacyTemplateFile) -> Result<(), TemplateError> {
        let archive = self.dir.join(IMPORTED_FOLDER);
        std::fs::create_dir_all(&archive).map_err(TemplateError::LegacyFolder)?;
        let name = file.path.file_name().unwrap_or_default();
        std::fs::rename(&file.path, archive.join(name)).map_err(TemplateError::LegacyFolder)
    }
}

/// The template in `path`, or `None` (logged) when it cannot be read.
fn parse(path: &Path) -> Option<LegacyTemplateFile> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let parsed = std::fs::read_to_string(path)
        .map_err(TemplateError::LegacyFolder)
        .and_then(|json| import_document(&json));
    match parsed {
        Ok(content) => Some(LegacyTemplateFile {
            path: path.to_owned(),
            content,
        }),
        Err(error) => {
            log::warn!(target: "templates", "skipped the legacy template file {name}: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY: &str = r#"{ "name": "Mine", "description": "d",
        "sections": [{ "title": "Notes", "instruction": "All of it", "format": "paragraph" }] }"#;

    #[test]
    fn parsed_files_are_listed_and_archived_and_others_stay() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mine.json"), LEGACY).unwrap();
        std::fs::write(dir.path().join("broken.json"), "{").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "x").unwrap();
        let folder = LegacyTemplateFolder::new(dir.path());

        let files = folder.templates().unwrap();
        folder.archive(&files[0]).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].content.name, "Mine");
        assert!(dir.path().join("imported/mine.json").exists());
        assert!(dir.path().join("broken.json").exists());
        assert!(folder.templates().unwrap().is_empty());
    }

    #[test]
    fn a_missing_folder_has_no_templates() {
        let folder = LegacyTemplateFolder::new("/nonexistent/deino/templates");

        assert!(folder.templates().unwrap().is_empty());
    }
}
