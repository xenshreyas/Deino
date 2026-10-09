//! Business rules for meetings.

use std::sync::Arc;

use deino_domain::{Clock, MeetingId};

use super::{Meeting, MeetingError, MeetingRepo, MeetingTitle};

/// Lists, opens and renames meetings.
pub struct MeetingService {
    repo: Arc<dyn MeetingRepo>,
    clock: Arc<dyn Clock>,
}

impl MeetingService {
    /// Creates the service over its ports.
    pub fn new(repo: Arc<dyn MeetingRepo>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    /// Every meeting, newest first.
    pub async fn list(&self) -> Result<Vec<Meeting>, MeetingError> {
        Ok(self.repo.list().await?)
    }

    /// The meeting with `id`.
    pub async fn get(&self, id: &MeetingId) -> Result<Meeting, MeetingError> {
        self.repo
            .find(id)
            .await?
            .ok_or_else(|| MeetingError::NotFound(id.clone()))
    }

    /// Gives the meeting a new title and returns the renamed meeting.
    ///
    /// The title is trimmed and must be non-empty and at most
    /// [`MAX_TITLE_CHARS`](super::MAX_TITLE_CHARS) characters long.
    pub async fn rename(&self, id: &MeetingId, title: &str) -> Result<Meeting, MeetingError> {
        let title = MeetingTitle::parse(title)?;
        let renamed = self.repo.rename(id, &title, self.clock.now()).await?;
        let meeting = renamed.ok_or_else(|| MeetingError::NotFound(id.clone()))?;
        // The ID only: titles are meeting content and never reach the log.
        log::info!(target: "meetings", "renamed meeting {id}");
        Ok(meeting)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;
    use crate::meetings::FakeMeetingRepo;

    const SAVED_AT: &str = "2026-10-08T09:00:00Z";

    fn meeting(id: &str, title: &str, created_at: &str) -> Meeting {
        Meeting {
            id: MeetingId::new(id),
            title: title.into(),
            created_at: at(created_at),
            updated_at: at(created_at),
        }
    }

    fn service(meetings: Vec<Meeting>) -> (MeetingService, Arc<FakeClock>) {
        let clock = Arc::new(FakeClock::new(at(SAVED_AT)));
        let repo = Arc::new(FakeMeetingRepo::with(meetings));
        (MeetingService::new(repo, clock.clone()), clock)
    }

    #[tokio::test]
    async fn list_returns_meetings_newest_first() {
        let (service, _) = service(vec![
            meeting("meeting-old", "Old", "2026-10-01T09:00:00Z"),
            meeting("meeting-new", "New", SAVED_AT),
        ]);

        let titles: Vec<String> = service
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|meeting| meeting.title)
            .collect();

        assert_eq!(titles, ["New", "Old"]);
    }

    #[tokio::test]
    async fn get_of_an_unknown_id_is_not_found() {
        let (service, _) = service(vec![]);

        let error = service.get(&MeetingId::new("meeting-x")).await.unwrap_err();

        assert!(matches!(error, MeetingError::NotFound(id) if id.as_str() == "meeting-x"));
    }

    #[tokio::test]
    async fn rename_trims_the_title_and_stamps_the_clock_time() {
        let (service, clock) = service(vec![meeting("meeting-a", "Standup", SAVED_AT)]);
        clock.advance(TimeDelta::hours(2));

        let renamed = service
            .rename(&MeetingId::new("meeting-a"), "  Planning  ")
            .await
            .unwrap();

        assert_eq!(renamed.title, "Planning");
        assert_eq!(renamed.updated_at, at(SAVED_AT) + TimeDelta::hours(2));
        assert_eq!(service.get(&renamed.id).await.unwrap(), renamed);
    }

    #[tokio::test]
    async fn rename_rejects_a_blank_title_before_touching_storage() {
        let (service, _) = service(vec![meeting("meeting-a", "Standup", SAVED_AT)]);

        let error = service
            .rename(&MeetingId::new("meeting-a"), "   ")
            .await
            .unwrap_err();

        assert!(matches!(error, MeetingError::EmptyTitle));
        let unchanged = service.get(&MeetingId::new("meeting-a")).await.unwrap();
        assert_eq!(unchanged.title, "Standup");
    }

    #[tokio::test]
    async fn rename_of_an_unknown_id_is_not_found() {
        let (service, _) = service(vec![]);

        let error = service
            .rename(&MeetingId::new("meeting-x"), "Planning")
            .await
            .unwrap_err();

        assert!(matches!(error, MeetingError::NotFound(_)));
    }
}
