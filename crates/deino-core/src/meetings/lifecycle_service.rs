//! Moves meetings through their lifecycle.
//!
//! The rules live in [`MeetingStatus::apply`](deino_domain::MeetingStatus::apply);
//! this service reads the current status, asks the state machine for the
//! next one and stores it with a compare-and-set, so a concurrent writer
//! (the recorder and the pipeline, say) can never be silently overwritten.

use std::sync::Arc;

use deino_domain::{Clock, MeetingEvent, MeetingId, MeetingStatus};

use super::{MeetingError, MeetingLifecycleRepo, StatusChange};

/// Applies [`MeetingEvent`]s to stored meetings.
pub struct MeetingLifecycleService {
    repo: Arc<dyn MeetingLifecycleRepo>,
    clock: Arc<dyn Clock>,
}

impl MeetingLifecycleService {
    /// Creates the service over its ports.
    pub fn new(repo: Arc<dyn MeetingLifecycleRepo>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Applies `event` to the meeting and returns its new status.
    ///
    /// Fails with [`MeetingError::InvalidTransition`] when the event is not
    /// allowed in the meeting's status, and [`MeetingError::StatusChanged`]
    /// when another writer moved the meeting between the read and the write.
    pub async fn transition(
        &self,
        id: &MeetingId,
        event: MeetingEvent,
    ) -> Result<MeetingStatus, MeetingError> {
        let state = self
            .repo
            .state(id)
            .await?
            .ok_or_else(|| MeetingError::NotFound(id.clone()))?;
        let to = state.status.apply(event)?;
        let change = StatusChange {
            from: state.status,
            to,
            at: self.clock.now(),
        };
        if !self.repo.set_status(id, change).await? {
            return Err(MeetingError::StatusChanged(id.clone()));
        }
        log::info!(target: "meetings", "meeting {id} is now {to}");
        Ok(to)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use deino_domain::{FakeClock, MeetingSource};
    use deino_testkit::at;

    use super::*;
    use crate::meetings::{FakeMeetingRepo, MeetingTitle, NewMeeting};

    const CREATED_AT: &str = "2026-10-08T09:00:00Z";

    async fn service_with(
        status: MeetingStatus,
    ) -> (MeetingLifecycleService, Arc<FakeMeetingRepo>) {
        let repo = Arc::new(FakeMeetingRepo::default());
        repo.create(&NewMeeting {
            id: MeetingId::new("meeting-a"),
            title: MeetingTitle::parse("Standup").unwrap(),
            status,
            source: MeetingSource::Live,
            created_at: at(CREATED_AT),
            started_at: None,
            folder_path: None,
        })
        .await
        .unwrap();
        let clock = Arc::new(FakeClock::new(at(CREATED_AT) + TimeDelta::hours(1)));
        (MeetingLifecycleService::new(repo.clone(), clock), repo)
    }

    #[tokio::test]
    async fn stopping_a_recording_moves_it_to_processing() {
        let (service, repo) = service_with(MeetingStatus::Recording).await;
        let id = MeetingId::new("meeting-a");

        let status = service
            .transition(&id, MeetingEvent::StopRecording)
            .await
            .unwrap();

        assert_eq!(status, MeetingStatus::Processing);
        let stored = repo.state(&id).await.unwrap().unwrap();
        assert_eq!(stored.status, MeetingStatus::Processing);
    }

    #[tokio::test]
    async fn an_illegal_event_leaves_the_meeting_untouched() {
        let (service, repo) = service_with(MeetingStatus::Ready).await;
        let id = MeetingId::new("meeting-a");

        let error = service
            .transition(&id, MeetingEvent::StopRecording)
            .await
            .unwrap_err();

        assert!(matches!(error, MeetingError::InvalidTransition(_)));
        let stored = repo.state(&id).await.unwrap().unwrap();
        assert_eq!(stored.status, MeetingStatus::Ready);
    }

    #[tokio::test]
    async fn an_unknown_meeting_is_not_found() {
        let (service, _) = service_with(MeetingStatus::Ready).await;

        let error = service
            .transition(&MeetingId::new("meeting-x"), MeetingEvent::Reprocess)
            .await
            .unwrap_err();

        assert!(matches!(error, MeetingError::NotFound(_)));
    }
}
