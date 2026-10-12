//! What the UI sees of an import: the preview count and the progress.

use deino_domain::ImportSource;
use serde::Serialize;

/// What an import would add, counted before it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    /// Where the meetings come from.
    pub source: ImportSource,
    /// Every meeting the source lists.
    pub total: u32,
    /// Those already imported, which an import skips.
    pub already_imported: u32,
    /// Those an import would add.
    pub new: u32,
}

/// Where the import job is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ImportPhase {
    /// Nothing has run since the app started.
    Idle,
    /// Listing the source's meetings.
    Listing,
    /// Importing them one by one.
    Importing,
    /// Every new meeting was tried (some may have failed).
    Finished,
    /// The user stopped it; what was imported stays.
    Cancelled,
    /// It stopped on an error (`error` says which).
    Failed,
}

impl ImportPhase {
    /// True while the job runs.
    pub const fn is_running(self) -> bool {
        matches!(self, Self::Listing | Self::Importing)
    }
}

/// The import job's progress, sent as it changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    /// The source being imported, once a job ran.
    pub source: Option<ImportSource>,
    /// Where the job is.
    pub phase: ImportPhase,
    /// New meetings to import (known once listing ends).
    pub total: u32,
    /// Meetings imported so far.
    pub imported: u32,
    /// Meetings skipped because they were imported before.
    pub skipped: u32,
    /// Meetings that could not be imported.
    pub failed: u32,
    /// Why the job failed, user-safe.
    pub error: Option<String>,
}

impl ImportProgress {
    /// No job yet.
    pub const fn idle() -> Self {
        Self {
            source: None,
            phase: ImportPhase::Idle,
            total: 0,
            imported: 0,
            skipped: 0,
            failed: 0,
            error: None,
        }
    }

    /// A job for `source` that is listing.
    pub fn listing(source: ImportSource) -> Self {
        Self {
            source: Some(source),
            phase: ImportPhase::Listing,
            ..Self::idle()
        }
    }
}

/// Receives the job's progress (the app emits it to the UI).
pub trait ImportEvents: Send + Sync {
    /// The job moved on.
    fn progress(&self, progress: &ImportProgress);
}

/// Drops progress: for tests and builds without a UI.
pub struct NoImportEvents;

impl ImportEvents for NoImportEvents {
    fn progress(&self, _progress: &ImportProgress) {}
}
