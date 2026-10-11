//! The start observer port: who hears that a recording started.
//!
//! [`RecordingService`](crate::RecordingService) calls its observer once a
//! recording runs and its meeting row exists. The app links the calendar
//! event nearest to now (ORG-03) and may rename the meeting after it; the
//! recorder then shows that title. The recorder never knows who listens.

use async_trait::async_trait;
use deino_domain::MeetingId;

/// Hears that a recording started.
#[async_trait]
pub trait StartObserver: Send + Sync {
    /// Called once `meeting` is recording. `titled` is true when the caller
    /// named the meeting, so an observer should keep that name. Returns the
    /// meeting's new title when the observer renamed it.
    ///
    /// The recording is already running and cannot be refused, so an
    /// observer handles and logs its own failures instead of returning them.
    async fn started(&self, meeting: &MeetingId, titled: bool) -> Option<String>;
}

/// Nobody listens: tests and builds without calendars.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoStartObserver;

#[async_trait]
impl StartObserver for NoStartObserver {
    async fn started(&self, _meeting: &MeetingId, _titled: bool) -> Option<String> {
        None
    }
}
