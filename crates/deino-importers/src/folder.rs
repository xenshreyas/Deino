//! A folder of exported meetings on this Mac: each `.json` file is one
//! meeting in Circleback's import shape (the local API's `POST /v1/meetings`
//! body), each `.md` file one meeting's notes. Files are keyed by the
//! SHA-256 of their bytes, so a file imports once however it is renamed,
//! and an edited file imports again as a new meeting.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use deino_core::importers::{Importer, ImporterError, SourceItem, SourcePage};
use deino_core::transcript_import::TranscriptImport;
use deino_domain::ImportSource;
use sha2::{Digest, Sha256};

use crate::mapping::Parts;

/// The most files one folder import reads.
pub const MAX_FILES: usize = 5_000;
/// The largest file read; bigger ones are skipped.
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Imports the JSON and Markdown files directly inside a folder.
pub struct FolderImporter {
    root: PathBuf,
}

impl FolderImporter {
    /// An importer over `root`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// The folder's meeting files, sorted by name.
    async fn files(&self) -> Result<Vec<PathBuf>, ImporterError> {
        let mut entries = tokio::fs::read_dir(&self.root)
            .await
            .map_err(ImporterError::Folder)?;
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(ImporterError::Folder)? {
            let path = entry.path();
            let small = entry
                .metadata()
                .await
                .is_ok_and(|meta| meta.is_file() && meta.len() <= MAX_FILE_BYTES);
            if small && kind(&path).is_some() {
                files.push(path);
            }
        }
        files.sort();
        files.truncate(MAX_FILES);
        Ok(files)
    }
}

#[async_trait]
impl Importer for FolderImporter {
    fn source(&self) -> ImportSource {
        ImportSource::Folder
    }

    async fn page(&self, _cursor: Option<&str>) -> Result<SourcePage, ImporterError> {
        let mut items = Vec::new();
        for path in self.files().await? {
            let Ok(bytes) = tokio::fs::read(&path).await else {
                continue;
            };
            let id = format!("{:x}", Sha256::digest(&bytes));
            items.push(match parse(&path, &bytes) {
                Some(meeting) => SourceItem::complete(id, meeting),
                // Counted as failed when imported, so the user hears of it.
                None => SourceItem::listed(id),
            });
        }
        Ok(SourcePage { items, next: None })
    }
}

/// What a file holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Json,
    Markdown,
}

fn kind(path: &Path) -> Option<Kind> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "json" => Some(Kind::Json),
        "md" | "markdown" => Some(Kind::Markdown),
        _ => None,
    }
}

/// The file's meeting, or `None` when it is not one.
fn parse(path: &Path, bytes: &[u8]) -> Option<TranscriptImport> {
    match kind(path)? {
        Kind::Json => serde_json::from_slice(bytes).ok(),
        Kind::Markdown => {
            let text = std::str::from_utf8(bytes).ok()?;
            let stem = path.file_stem().and_then(|stem| stem.to_str());
            Some(markdown(text, stem))
        }
    }
}

/// A Markdown note: `title:` and `date:` from front matter, else the first
/// `# ` heading and the file's name; the rest is the notes.
fn markdown(text: &str, stem: Option<&str>) -> TranscriptImport {
    let (front, body) = split_front_matter(text);
    let field = |name: &str| {
        front.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            let value = value.trim().trim_matches(['"', '\'']);
            (key.trim() == name && !value.is_empty()).then(|| value.to_owned())
        })
    };
    let heading = body
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|title| title.trim().to_owned());
    let notes = match &heading {
        Some(title) => body.replacen(&format!("# {title}"), "", 1),
        None => body.to_owned(),
    };
    Parts {
        title: field("title")
            .or(heading)
            .or_else(|| stem.map(str::to_owned)),
        created_at: field("date").as_deref().and_then(date),
        notes: Some(notes.trim().to_owned()),
        ..Parts::default()
    }
    .into_import()
}

/// Front matter between `---` lines at the very top, and the rest.
fn split_front_matter(text: &str) -> (&str, &str) {
    let Some(rest) = text.strip_prefix("---\n") else {
        return ("", text);
    };
    match rest.split_once("\n---") {
        Some((front, body)) => (front, body.trim_start_matches(['-', '\n'])),
        None => ("", text),
    }
}

/// An RFC 3339 time or a plain `YYYY-MM-DD` day (as its UTC noon).
fn date(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(at) = DateTime::parse_from_rfc3339(value) {
        return Some(at.to_utc());
    }
    let day = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    Some(day.and_hms_opt(12, 0, 0)?.and_utc())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_takes_its_title_and_date_from_front_matter_or_heading() {
        let note = markdown(
            "---\ntitle: \"Board prep\"\ndate: 2026-03-04\n---\n# Ignored\n\nBody",
            Some("file"),
        );
        assert_eq!(note.name, "Board prep");
        assert_eq!(note.created_at, date("2026-03-04T12:00:00Z"));

        let plain = markdown("# Standup\n\n- shipped", Some("file"));
        assert_eq!(plain.name, "Standup");
        assert_eq!(plain.notes.as_deref(), Some("- shipped"));
        assert!(plain.transcript.is_empty());

        assert_eq!(markdown("just text", Some("notes-0412")).name, "notes-0412");
    }
}
