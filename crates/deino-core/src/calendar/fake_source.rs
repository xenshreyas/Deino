//! A scripted [`CalendarSource`] and a recorder of [`CalendarEvents`], for
//! tests of the sync, the service and the app's plugin.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use deino_domain::CalendarProvider;

use super::{
    CalendarAccess, CalendarEvents, CalendarSource, CalendarStatus, SourceError, SourceSnapshot,
    SyncWindow,
};
use crate::testing::lock;

/// A source whose access and contents a test sets. It answers the access
/// prompt with a scripted choice, and records each window it was read for.
#[derive(Debug)]
pub struct FakeCalendarSource {
    provider: CalendarProvider,
    access: Mutex<CalendarAccess>,
    answer: Mutex<CalendarAccess>,
    snapshot: Mutex<SourceSnapshot>,
    failing: Mutex<bool>,
    windows: Mutex<Vec<SyncWindow>>,
    fetches: AtomicUsize,
}

impl FakeCalendarSource {
    /// An empty source of `provider` that has not been granted access and
    /// grants it when asked.
    pub fn new(provider: CalendarProvider) -> Self {
        Self {
            provider,
            access: Mutex::new(CalendarAccess::NotDetermined),
            answer: Mutex::new(CalendarAccess::Granted),
            snapshot: Mutex::new(SourceSnapshot::default()),
            failing: Mutex::new(false),
            windows: Mutex::new(Vec::new()),
            fetches: AtomicUsize::new(0),
        }
    }

    /// Sets the current access.
    pub fn set_access(&self, access: CalendarAccess) {
        *lock(&self.access) = access;
    }

    /// Sets what the user answers when asked for access.
    pub fn answer_prompt_with(&self, access: CalendarAccess) {
        *lock(&self.answer) = access;
    }

    /// Sets what the source holds.
    pub fn set_snapshot(&self, snapshot: SourceSnapshot) {
        *lock(&self.snapshot) = snapshot;
    }

    /// Makes every read fail (true) or succeed (false).
    pub fn set_failing(&self, failing: bool) {
        *lock(&self.failing) = failing;
    }

    /// How many times the source was read.
    pub fn fetch_count(&self) -> usize {
        self.fetches.load(Ordering::SeqCst)
    }

    /// The windows the source was read for, in order.
    pub fn windows(&self) -> Vec<SyncWindow> {
        lock(&self.windows).clone()
    }
}

#[async_trait]
impl CalendarSource for FakeCalendarSource {
    fn provider(&self) -> CalendarProvider {
        self.provider
    }

    async fn access(&self) -> CalendarAccess {
        *lock(&self.access)
    }

    async fn request_access(&self) -> Result<CalendarAccess, SourceError> {
        let mut access = lock(&self.access);
        if *access == CalendarAccess::NotDetermined {
            *access = *lock(&self.answer);
        }
        Ok(*access)
    }

    async fn fetch(&self, window: SyncWindow) -> Result<SourceSnapshot, SourceError> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        lock(&self.windows).push(window);
        if *lock(&self.failing) {
            return Err(SourceError::Failed("scripted failure".into()));
        }
        Ok(lock(&self.snapshot).clone())
    }
}

/// Records every `CalendarChanged` status, in order.
#[derive(Debug, Default)]
pub struct RecordedCalendarEvents {
    statuses: Mutex<Vec<CalendarStatus>>,
}

impl RecordedCalendarEvents {
    /// Every status announced so far.
    pub fn statuses(&self) -> Vec<CalendarStatus> {
        lock(&self.statuses).clone()
    }
}

impl CalendarEvents for RecordedCalendarEvents {
    fn changed(&self, status: &CalendarStatus) {
        lock(&self.statuses).push(status.clone());
    }
}
