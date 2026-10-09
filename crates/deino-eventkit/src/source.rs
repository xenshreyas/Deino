//! [`EventKitSource`]: the `CalendarSource` over macOS Calendar.

use std::time::Duration;

use async_trait::async_trait;
use deino_core::calendar::{
    CalendarAccess, CalendarSource, SourceError, SourceSnapshot, SyncTriggers, SyncWindow,
};
use deino_domain::CalendarProvider;

/// The longest one read of the store may take before the sync gives up (a
/// month and a half of events reads in well under a second).
pub const FETCH_TIMEOUT: Duration = Duration::from_mins(1);

/// macOS Calendar, read through `EventKit` on its own thread.
#[derive(Debug)]
pub struct EventKitSource {
    #[cfg(target_os = "macos")]
    worker: Option<crate::worker::Worker>,
}

impl EventKitSource {
    /// Starts the `EventKit` thread. A thread that cannot start leaves the
    /// source reporting failures, never panicking.
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            let worker = crate::worker::Worker::spawn()
                .inspect_err(|error| log::error!(target: "calendar", "could not start the EventKit thread: {error}"))
                .ok();
            Self { worker }
        }
        #[cfg(not(target_os = "macos"))]
        Self {}
    }

    /// Nudges `triggers` whenever macOS Calendar changes or the Mac wakes.
    pub fn watch(&self, triggers: SyncTriggers) {
        #[cfg(target_os = "macos")]
        self.send(crate::worker::Request::Watch(triggers));
        #[cfg(not(target_os = "macos"))]
        drop(triggers);
    }

    /// Hands a request to the thread; false when it is not running.
    #[cfg(target_os = "macos")]
    fn send(&self, request: crate::worker::Request) -> bool {
        self.worker
            .as_ref()
            .is_some_and(|worker| worker.send(request))
    }
}

impl Default for EventKitSource {
    fn default() -> Self {
        Self::new()
    }
}

/// The error for a thread that stopped before it answered.
#[cfg(target_os = "macos")]
fn thread_gone() -> SourceError {
    SourceError::Failed("the EventKit thread is not running".into())
}

#[async_trait]
impl CalendarSource for EventKitSource {
    fn provider(&self) -> CalendarProvider {
        CalendarProvider::Apple
    }

    async fn access(&self) -> CalendarAccess {
        #[cfg(target_os = "macos")]
        return crate::store::authorization();
        #[cfg(not(target_os = "macos"))]
        CalendarAccess::Unsupported
    }

    #[cfg(target_os = "macos")]
    async fn request_access(&self) -> Result<CalendarAccess, SourceError> {
        let (reply, answer) = tokio::sync::oneshot::channel();
        if !self.send(crate::worker::Request::Prompt(reply)) {
            return Err(thread_gone());
        }
        // No timeout: the user may take their time with the prompt.
        answer.await.map_err(|_| thread_gone())
    }

    #[cfg(target_os = "macos")]
    async fn fetch(&self, window: SyncWindow) -> Result<SourceSnapshot, SourceError> {
        let (reply, snapshot) = tokio::sync::oneshot::channel();
        if !self.send(crate::worker::Request::Fetch(window, reply)) {
            return Err(thread_gone());
        }
        match tokio::time::timeout(FETCH_TIMEOUT, snapshot).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(thread_gone()),
            Err(_) => Err(SourceError::Failed(
                "reading macOS Calendar timed out".into(),
            )),
        }
    }

    #[cfg(target_os = "macos")]
    fn open_access_settings(&self) -> bool {
        crate::privacy::open_calendars_page()
    }

    #[cfg(not(target_os = "macos"))]
    async fn request_access(&self) -> Result<CalendarAccess, SourceError> {
        Ok(CalendarAccess::Unsupported)
    }

    #[cfg(not(target_os = "macos"))]
    async fn fetch(&self, _window: SyncWindow) -> Result<SourceSnapshot, SourceError> {
        Err(SourceError::Unsupported)
    }
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn off_macos_the_source_is_unsupported() {
        let source = EventKitSource::new();
        assert_eq!(source.access().await, CalendarAccess::Unsupported);
        assert!(matches!(
            source
                .fetch(SyncWindow::around(chrono::DateTime::UNIX_EPOCH))
                .await,
            Err(SourceError::Unsupported)
        ));
    }
}
