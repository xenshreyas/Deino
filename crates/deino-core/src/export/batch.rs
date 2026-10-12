//! [`ExportService::save_many`]: the Meetings page's bulk export (ORG-07).
//! The user picks a folder once, then each meeting is assembled, rendered
//! and written there under the name a single export would suggest, exactly
//! as [`ExportService::save`] does for one meeting.
//!
//! The folder is the user's, so nothing in it is overwritten: a name that is
//! taken (by an earlier file or by two meetings with the same title and day)
//! gets " (2)", " (3)" and so on.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use deino_domain::MeetingId;
use deino_export::{ExportFormat, RenderOptions};

use super::service::{rendered, suggested_name};
use super::{ExportError, ExportService, FolderDialog, SavedExports};

/// How many numbered names to try before giving up on a free one.
const MAX_NUMBERED_NAMES: u32 = 1_000;

impl ExportService {
    /// Asks for a folder with `dialog`, then writes each of `meetings` there
    /// as `format` with the default sections. `None` when the user
    /// cancelled. Stops at the first meeting that fails; the files already
    /// written stay.
    pub async fn save_many(
        &self,
        meetings: &[MeetingId],
        format: ExportFormat,
        dialog: &dyn FolderDialog,
    ) -> Result<Option<SavedExports>, ExportError> {
        let Some(folder) = dialog.pick_folder().await else {
            return Ok(None);
        };
        let mut written = HashSet::new();
        for meeting in meetings {
            let export = self.deps.assembler.assemble(meeting).await?;
            let path = self
                .free_path(&folder, &suggested_name(&export, format), &written)
                .await?;
            let file = rendered(format, export, RenderOptions::default()).await?;
            self.deps
                .vault
                .write(&path, file.bytes)
                .await
                .map_err(ExportError::Write)?;
            written.insert(path);
        }
        log::info!(target: "export", "exported {} meetings into a folder", written.len());
        Ok(Some(SavedExports {
            count: u32::try_from(written.len()).unwrap_or(u32::MAX),
            folder: folder_name(&folder),
        }))
    }

    /// `folder/name`, numbered when that file exists or this batch wrote it.
    async fn free_path(
        &self,
        folder: &Path,
        name: &str,
        written: &HashSet<PathBuf>,
    ) -> Result<PathBuf, ExportError> {
        for number in 1..=MAX_NUMBERED_NAMES {
            let path = folder.join(numbered(name, number));
            if written.contains(&path) {
                continue;
            }
            let existing = self.deps.vault.head(&path).await;
            if existing.map_err(ExportError::Write)?.is_none() {
                return Ok(path);
            }
        }
        Err(ExportError::Write(std::io::Error::other(
            "no free file name in the folder",
        )))
    }
}

/// "Standup.md", then "Standup (2).md", "Standup (3).md", …
fn numbered(name: &str, number: u32) -> String {
    if number == 1 {
        return name.to_owned();
    }
    match name.rsplit_once('.') {
        Some((stem, extension)) => format!("{stem} ({number}).{extension}"),
        None => format!("{name} ({number})"),
    }
}

/// The folder's own name, for the confirmation.
fn folder_name(folder: &Path) -> String {
    folder.file_name().map_or_else(
        || folder.to_string_lossy().into_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
