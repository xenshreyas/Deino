//! [`CalendarSync`]: one sync of one source, from reading to storing.
//!
//! Reads the source's [`SyncWindow`] around now, turns it into a
//! [`SyncBatch`] (`mapping.rs`), stores the batch in one transaction and
//! then adds the attendees it met to people and companies. The attendee
//! step is best effort: a failure there is logged and never fails the sync,
//! whose events are already stored.

use std::sync::Arc;

use deino_domain::{Clock, Email};

use super::batch::{SyncBatch, SyncReport};
use super::entities::AttendeeDirectory;
use super::mapping::{BatchContext, to_batch};
use super::{CalendarError, CalendarSource, CalendarSyncRepo, SyncWindow};

/// Syncs a source into the store.
pub struct CalendarSync {
    repo: Arc<dyn CalendarSyncRepo>,
    directory: AttendeeDirectory,
    clock: Arc<dyn Clock>,
}

impl CalendarSync {
    /// A sync engine storing through `repo` and adding attendees to `directory`.
    pub fn new(
        repo: Arc<dyn CalendarSyncRepo>,
        directory: AttendeeDirectory,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            directory,
            clock,
        }
    }

    /// Reads `source` around now and stores what it has. `profile_emails`
    /// (Settings → Profile) and the user's own entries in the invitations
    /// tell internal meetings from external ones.
    pub async fn run(
        &self,
        source: &dyn CalendarSource,
        profile_emails: &[Email],
    ) -> Result<SyncReport, CalendarError> {
        let window = SyncWindow::around(self.clock.now());
        let snapshot = source.fetch(window).await?;
        if snapshot.accounts.is_empty() {
            // A store that is still loading (right after access is granted)
            // reports no accounts; storing that would delete every account
            // and with it the events meetings link to.
            log::info!(target: "calendar", "{} reported no accounts; nothing stored", source.provider());
            return Ok(SyncReport::default());
        }
        let context = BatchContext {
            provider: source.provider(),
            window,
            clock: self.clock.as_ref(),
            profile_emails,
        };
        let (batch, own) = to_batch(&snapshot, &context);
        let report = self.repo.apply(&batch).await?;
        match self.directory.record(events_of(&batch), &own).await {
            Ok(added) if added > 0 => {
                log::info!(target: "calendar", "added {added} people from calendar attendees");
            }
            Ok(_) => {}
            Err(error) => {
                log::warn!(target: "calendar", "could not add calendar attendees to people: {error}");
            }
        }
        log::debug!(
            target: "calendar",
            "synced {}: {} added, {} updated, {} removed, {} calendar changes",
            source.provider(),
            report.events_added,
            report.events_updated,
            report.events_removed,
            report.calendars_changed,
        );
        Ok(report)
    }
}

/// Every event of `batch`.
fn events_of(batch: &SyncBatch) -> impl Iterator<Item = &super::batch::SyncedEvent> {
    batch
        .accounts
        .iter()
        .flat_map(|account| &account.calendars)
        .flat_map(|calendar| &calendar.events)
}
