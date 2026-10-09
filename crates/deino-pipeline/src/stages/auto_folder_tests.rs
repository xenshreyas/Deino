//! `auto_folder` over the real auto-filer and SQLite, with the seeded
//! fixture meetings (ORG-05's acceptance: a recurring rule files the next
//! instance of a series).

use std::sync::Arc;

use deino_core::folders::{
    AutoFileService, FilingService, Folder, FolderPorts, FolderRule, FolderService, NewFolder,
    SqliteFolderRepo,
};
use deino_core::meetings::{
    MeetingLifecycleRepo, MeetingLink, MeetingTitle, NewMeeting, SqliteMeetingRepo,
};
use deino_db::{Database, test_db};
use deino_domain::{FakeClock, MeetingId, MeetingSource, MeetingStatus, SeriesId, StageId};
use deino_testkit::at;
use deino_testkit::fixture::{Fixtures, MeetingKind, seed_fixtures};
use tokio_util::sync::CancellationToken;

use super::auto_folder::AutoFolderStage;
use crate::{PipelineStage, RecordedProgress, StageCtx, StageError, StageOutcome, StagePorts};

const NOW: &str = "2026-10-09T09:00:00Z";

struct Harness {
    database: Database,
    ports: FolderPorts,
    stage: AutoFolderStage,
}

async fn harness() -> Harness {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let repo = Arc::new(SqliteFolderRepo::new(database.clone()));
    let ports = FolderPorts::over(repo, Arc::new(FakeClock::new(at(NOW))));
    let mut stage_ports = StagePorts::fakes();
    stage_ports.auto_filer = Arc::new(AutoFileService::new(&ports));
    Harness {
        database,
        ports,
        stage: AutoFolderStage::new(&stage_ports),
    }
}

fn ctx(cancel: CancellationToken) -> StageCtx {
    StageCtx {
        attempt: 1,
        cancel,
        clock: Arc::new(FakeClock::new(at(NOW))),
        events: Arc::new(RecordedProgress::default()),
    }
}

impl Harness {
    async fn folder(&self, name: &str) -> Folder {
        let new = NewFolder {
            name: name.into(),
            parent_id: None,
        };
        FolderService::new(&self.ports).create(&new).await.unwrap()
    }

    async fn series_of(&self, meeting: &MeetingId) -> SeriesId {
        let meetings = SqliteMeetingRepo::new(self.database.clone());
        let state = meetings.state(meeting).await.unwrap().unwrap();
        state.series_id.expect("the fixture is a recurring series")
    }

    /// Records the next meeting of `series`, now in `processing`.
    async fn next_instance(&self, series: SeriesId) -> MeetingId {
        let meetings = SqliteMeetingRepo::new(self.database.clone());
        let id = MeetingId::new("meeting-next");
        let new = NewMeeting {
            id: id.clone(),
            title: MeetingTitle::parse("Eng standup").unwrap(),
            status: MeetingStatus::Processing,
            source: MeetingSource::Live,
            created_at: at(NOW),
            started_at: Some(at(NOW)),
            folder_path: None,
        };
        meetings.create(&new).await.unwrap();
        let link = MeetingLink {
            calendar_event_id: None,
            series_id: Some(series),
        };
        meetings.link(&id, &link, at(NOW)).await.unwrap();
        id
    }

    async fn run(&self, meeting: &MeetingId) -> Result<StageOutcome, StageError> {
        self.stage
            .run(&ctx(CancellationToken::new()), meeting.clone())
            .await
    }
}

#[tokio::test]
async fn the_next_meeting_of_a_collected_fixture_series_is_filed() {
    let harness = harness().await;
    let fixtures = Fixtures::load().unwrap();
    let standup = &fixtures.meeting(MeetingKind::Standup).unwrap().id;
    let standups = harness.folder("Standups").await;
    let filing = FilingService::new(&harness.ports);
    let offer = filing
        .add_meeting(&standups.id, standup)
        .await
        .unwrap()
        .series_offer;
    let series_id = offer.expect("the standup offers its series");
    filing
        .add_rule(&standups.id, FolderRule::Series { series_id })
        .await
        .unwrap();
    let next = harness
        .next_instance(harness.series_of(standup).await)
        .await;

    let outcome = harness.run(&next).await;
    let rerun = harness.run(&next).await;

    assert_eq!(outcome, Ok(StageOutcome::Done));
    assert_eq!(rerun, Ok(StageOutcome::Done));
    let filed: Vec<MeetingId> = filing
        .meetings(&standups.id)
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect();
    assert_eq!(filed, [next, standup.clone()]);
}

#[tokio::test]
async fn nothing_to_file_is_a_skip_with_a_reason() {
    let harness = harness().await;
    let fixtures = Fixtures::load().unwrap();
    let planning = &fixtures.meeting(MeetingKind::Planning).unwrap().id;

    let no_rules = harness.run(planning).await;
    let customers = harness.folder("Customers").await;
    let domain = FolderRule::AttendeeDomain {
        domain: "nowhere.example".into(),
    };
    FilingService::new(&harness.ports)
        .add_rule(&customers.id, domain)
        .await
        .unwrap();
    let no_match = harness.run(planning).await;
    let gone = harness.run(&MeetingId::new("meeting-x")).await;

    assert!(matches!(no_rules, Ok(StageOutcome::Skipped(reason)) if reason.contains("No folder")));
    assert!(matches!(no_match, Ok(StageOutcome::Skipped(reason)) if reason.contains("match")));
    assert!(matches!(gone, Ok(StageOutcome::Skipped(reason)) if reason.contains("deleted")));
}

#[tokio::test]
async fn a_cancelled_run_files_nothing() {
    let harness = harness().await;
    let cancel = CancellationToken::new();
    cancel.cancel();

    let outcome = harness
        .stage
        .run(&ctx(cancel), MeetingId::new("meeting-x"))
        .await;

    assert_eq!(outcome, Err(StageError::Cancelled));
}

#[tokio::test]
async fn the_stage_is_auto_folder_at_order_seventy_five() {
    let harness = harness().await;
    assert_eq!(
        (harness.stage.id(), harness.stage.order()),
        (StageId::new("auto_folder"), 75)
    );
}
