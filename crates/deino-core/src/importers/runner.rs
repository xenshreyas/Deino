//! [`ImportRunner`]: pages through a source, skips what the ledger has, and
//! imports the rest through DEV-03's [`TranscriptImporter`], the one import
//! path, so imported meetings are indexed for search and chat like any other.

use std::collections::HashSet;
use std::sync::Arc;

use deino_domain::{Clock, ErrorClass, MeetingId};
use tokio_util::sync::CancellationToken;

use super::{
    ImportEvents, ImportLedger, ImportPhase, ImportPreview, ImportProgress, Importer,
    ImporterError, LedgerEntry, SourceItem,
};
use crate::transcript_import::TranscriptImporter;

/// The most pages one listing reads, against a source whose cursor never ends.
pub const MAX_PAGES: usize = 2_000;

/// What a listing found.
#[derive(Default)]
struct Listing {
    /// New meetings, in the source's order.
    pending: Vec<SourceItem>,
    /// Every distinct meeting listed.
    total: u32,
    /// Those the ledger already has.
    skipped: u32,
}

/// Previews and runs imports from any [`Importer`].
pub struct ImportRunner {
    importer: Arc<TranscriptImporter>,
    ledger: Arc<dyn ImportLedger>,
    clock: Arc<dyn Clock>,
}

impl ImportRunner {
    /// A runner writing through `importer` and recording in `ledger`.
    pub fn new(
        importer: Arc<TranscriptImporter>,
        ledger: Arc<dyn ImportLedger>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            importer,
            ledger,
            clock,
        }
    }

    /// Counts what importing `source` would add, without importing.
    pub async fn preview(&self, source: &dyn Importer) -> Result<ImportPreview, ImporterError> {
        let listing = self.list(source, &CancellationToken::new()).await?;
        Ok(ImportPreview {
            source: source.source(),
            total: listing.total,
            already_imported: listing.skipped,
            new: listing.total - listing.skipped,
        })
    }

    /// Imports every new meeting of `source`, reporting progress to
    /// `events`, until done, cancelled or stopped by an error. One meeting
    /// that fails is counted and skipped; it is tried again next time.
    pub async fn run(
        &self,
        source: &dyn Importer,
        cancel: &CancellationToken,
        events: &dyn ImportEvents,
    ) -> ImportProgress {
        let mut progress = ImportProgress::listing(source.source());
        events.progress(&progress);
        let listing = match self.list(source, cancel).await {
            Ok(listing) => listing,
            Err(error) => return failed(progress, &error, events),
        };
        progress.phase = ImportPhase::Importing;
        progress.total = u32::try_from(listing.pending.len()).unwrap_or(u32::MAX);
        progress.skipped = listing.skipped;
        events.progress(&progress);
        for item in listing.pending {
            if cancel.is_cancelled() {
                break;
            }
            match self.import_one(source, item).await {
                Ok(_) => progress.imported += 1,
                Err(error) if error.stops_the_run() => return failed(progress, &error, events),
                Err(error) => {
                    log::warn!(target: "importers", "a {} meeting was not imported ({:?})", source.source(), error.code());
                    progress.failed += 1;
                }
            }
            events.progress(&progress);
        }
        progress.phase = if cancel.is_cancelled() {
            ImportPhase::Cancelled
        } else {
            ImportPhase::Finished
        };
        events.progress(&progress);
        progress
    }

    /// Every page of `source`, split into new and already imported.
    async fn list(
        &self,
        source: &dyn Importer,
        cancel: &CancellationToken,
    ) -> Result<Listing, ImporterError> {
        let done = self.ledger.imported(source.source()).await?;
        let mut seen = HashSet::new();
        let mut listing = Listing::default();
        let mut cursor: Option<String> = None;
        for _ in 0..MAX_PAGES {
            if cancel.is_cancelled() {
                break;
            }
            let page = source.page(cursor.as_deref()).await?;
            for item in page
                .items
                .into_iter()
                .filter(|item| seen.insert(item.id.clone()))
            {
                listing.total += 1;
                if done.contains(&item.id) {
                    listing.skipped += 1;
                } else {
                    listing.pending.push(item);
                }
            }
            match page.next {
                Some(next) if cursor.as_ref() != Some(&next) => cursor = Some(next),
                _ => break,
            }
        }
        Ok(listing)
    }

    /// Fetches, writes and records one meeting.
    async fn import_one(
        &self,
        source: &dyn Importer,
        item: SourceItem,
    ) -> Result<MeetingId, ImporterError> {
        let source_id = item.id.clone();
        let meeting = source.fetch(item).await?;
        let id = self.importer.import(&meeting).await?;
        let entry = LedgerEntry {
            source: source.source(),
            source_id,
            meeting: id.clone(),
            at: self.clock.now(),
        };
        self.ledger.record(&entry).await?;
        Ok(id)
    }
}

/// `progress` stopped by `error`, reported.
fn failed(
    mut progress: ImportProgress,
    error: &ImporterError,
    events: &dyn ImportEvents,
) -> ImportProgress {
    log::warn!(target: "importers", "the import stopped ({:?})", error.code());
    progress.phase = ImportPhase::Failed;
    progress.error = Some(error.to_string());
    events.progress(&progress);
    progress
}

#[cfg(test)]
#[path = "runner_tests.rs"]
mod tests;
