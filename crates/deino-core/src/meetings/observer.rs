//! The lifecycle's observer port: who hears that a meeting changed status.
//!
//! [`MeetingLifecycleService`](super::MeetingLifecycleService) calls every
//! subscribed observer after a status change is stored. The post-meeting
//! pipeline (FND-08) subscribes to start when a meeting enters `processing`;
//! the service never knows who listens, so features that react to the
//! lifecycle plug in without editing it.

use async_trait::async_trait;
use deino_domain::MeetingId;

use super::StatusChange;

/// Hears about stored meeting status changes.
#[async_trait]
pub trait MeetingStatusObserver: Send + Sync {
    /// Called after `change` was stored for the meeting with `id`.
    ///
    /// The change is already committed and cannot be refused, so an observer
    /// handles and logs its own failures instead of returning them.
    async fn status_changed(&self, id: &MeetingId, change: StatusChange);
}
