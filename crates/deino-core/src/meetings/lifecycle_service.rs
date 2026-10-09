//! Moves meetings through their lifecycle.
//!
//! The rules live in [`MeetingStatus::apply`](deino_domain::MeetingStatus::apply);
//! this service reads the current status, asks the state machine for the
//! next one and stores it with a compare-and-set, so a concurrent writer
//! (the recorder and the pipeline, say) can never be silently overwritten.
//! Once a change is stored, every [`MeetingStatusObserver`] hears about it
//! (the pipeline starts when a meeting enters `processing`).

use std::sync::{Arc, PoisonError, RwLock};

use deino_domain::{Clock, MeetingEvent, MeetingId, MeetingStatus};

use super::{MeetingError, MeetingLifecycleRepo, MeetingStatusObserver, StatusChange};

/// Applies [`MeetingEvent`]s to stored meetings.
pub struct MeetingLifecycleService {
    repo: Arc<dyn MeetingLifecycleRepo>,
    clock: Arc<dyn Clock>,
    observers: RwLock<Vec<Arc<dyn MeetingStatusObserver>>>,
}

impl MeetingLifecycleService {
    /// Creates the service over its ports, with no observers.
    pub fn new(repo: Arc<dyn MeetingLifecycleRepo>, clock: Arc<dyn Clock>) -> Self {
        Self {
            repo,
            clock,
            observers: RwLock::default(),
        }
    }

    /// Adds `observer`, called after every stored status change from now on.
    ///
    /// Observers subscribe after construction because some of them (the
    /// pipeline) depend on this service themselves.
    pub fn subscribe(&self, observer: Arc<dyn MeetingStatusObserver>) {
        let mut observers = self
            .observers
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        observers.push(observer);
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
        self.notify(id, change).await;
        Ok(to)
    }

    /// Tells every observer about a stored change, in subscription order.
    async fn notify(&self, id: &MeetingId, change: StatusChange) {
        // Cloned so no lock is held while observers run (they may transition again).
        let observers = self
            .observers
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        for observer in observers {
            observer.status_changed(id, change).await;
        }
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

    /// Remembers every change it hears about.
    #[derive(Default)]
    struct RecordingObserver {
        heard: std::sync::Mutex<Vec<(MeetingId, StatusChange)>>,
    }

    #[async_trait::async_trait]
    impl MeetingStatusObserver for RecordingObserver {
        async fn status_changed(&self, id: &MeetingId, change: StatusChange) {
            self.heard.lock().unwrap().push((id.clone(), change));
        }
    }

    #[tokio::test]
    async fn observers_hear_each_stored_change_with_its_from_and_to() {
        let (service, _) = service_with(MeetingStatus::Recording).await;
        let observer = Arc::new(RecordingObserver::default());
        service.subscribe(observer.clone());
        let id = MeetingId::new("meeting-a");

        service
            .transition(&id, MeetingEvent::StopRecording)
            .await
            .unwrap();

        let heard = observer.heard.lock().unwrap().clone();
        assert_eq!(
            heard,
            [(
                id,
                StatusChange {
                    from: MeetingStatus::Recording,
                    to: MeetingStatus::Processing,
                    at: at(CREATED_AT) + TimeDelta::hours(1),
                }
            )]
        );
    }

    #[tokio::test]
    async fn a_refused_change_is_not_announced() {
        let (service, _) = service_with(MeetingStatus::Ready).await;
        let observer = Arc::new(RecordingObserver::default());
        service.subscribe(observer.clone());

        let refused = service
            .transition(&MeetingId::new("meeting-a"), MeetingEvent::Complete)
            .await;

        assert!(refused.is_err());
        assert!(observer.heard.lock().unwrap().is_empty());
    }
}
