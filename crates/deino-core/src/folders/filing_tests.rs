//! [`FilingService`] and [`AutoFileService`] over the fake ports: filing,
//! the series offer, rules and auto-filing.

use std::collections::BTreeSet;
use std::sync::Arc;

use deino_domain::{FakeClock, FolderId, MeetingId, SeriesId, TagId};
use deino_testkit::at;

use super::*;
use crate::meetings::Meeting;

const NOW: &str = "2026-10-09T09:00:00Z";
const SERIES: &str = "standup@google.com";

struct Harness {
    folders: FolderService,
    filing: FilingService,
    auto: AutoFileService,
    repo: Arc<FakeFolderRepo>,
}

fn meeting(id: &str) -> Meeting {
    Meeting {
        id: MeetingId::new(id),
        title: "Standup".into(),
        created_at: at(NOW),
        updated_at: at(NOW),
    }
}

fn harness() -> Harness {
    let repo = Arc::new(FakeFolderRepo::with_meetings(
        ["standup-1", "standup-2", "one-off"].map(meeting),
    ));
    let series = MeetingFacts {
        series_id: Some(SeriesId::new(SERIES)),
        ..MeetingFacts::default()
    };
    repo.set_facts(&MeetingId::new("standup-1"), series.clone());
    repo.set_facts(&MeetingId::new("standup-2"), series);
    let ports = FolderPorts::over(repo.clone(), Arc::new(FakeClock::new(at(NOW))));
    Harness {
        folders: FolderService::new(&ports),
        filing: FilingService::new(&ports),
        auto: AutoFileService::new(&ports),
        repo,
    }
}

impl Harness {
    async fn folder(&self, name: &str) -> Folder {
        let new = NewFolder {
            name: name.into(),
            parent_id: None,
        };
        self.folders.create(&new).await.unwrap()
    }
}

fn id(meeting: &str) -> MeetingId {
    MeetingId::new(meeting)
}

fn series_rule() -> FolderRule {
    FolderRule::Series {
        series_id: SeriesId::new(SERIES),
    }
}

#[tokio::test]
async fn filing_a_series_meeting_offers_the_series_until_the_folder_collects_it() {
    let harness = harness();
    let standups = harness.folder("Standups").await;

    let first = harness
        .filing
        .add_meeting(&standups.id, &id("standup-1"))
        .await
        .unwrap();
    harness
        .filing
        .add_rule(&standups.id, series_rule())
        .await
        .unwrap();
    let second = harness
        .filing
        .add_meeting(&standups.id, &id("standup-2"))
        .await
        .unwrap();

    assert_eq!(first.series_offer, Some(SeriesId::new(SERIES)));
    assert_eq!(second.series_offer, None);
    assert_eq!(
        harness.filing.meetings(&standups.id).await.unwrap().len(),
        2
    );
}

#[tokio::test]
async fn a_one_off_meeting_offers_nothing_and_can_sit_in_many_folders() {
    let harness = harness();
    let (work, home) = (harness.folder("Work").await, harness.folder("Home").await);

    let filed = harness
        .filing
        .add_meeting(&work.id, &id("one-off"))
        .await
        .unwrap();
    harness
        .filing
        .add_meeting(&home.id, &id("one-off"))
        .await
        .unwrap();
    harness
        .filing
        .add_meeting(&home.id, &id("one-off"))
        .await
        .unwrap();

    assert_eq!(filed.series_offer, None);
    let names: Vec<String> = harness
        .filing
        .folders_of(&id("one-off"))
        .await
        .unwrap()
        .into_iter()
        .map(|folder| folder.name)
        .collect();
    assert_eq!(names, ["Home", "Work"]);
    assert_eq!(harness.filing.unfiled().await.unwrap().len(), 2);
}

#[tokio::test]
async fn filing_needs_an_existing_folder_and_meeting() {
    let harness = harness();
    let work = harness.folder("Work").await;
    let missing_folder = FolderId::generate(&FakeClock::new(at(NOW)));

    let no_meeting = harness.filing.add_meeting(&work.id, &id("meeting-x")).await;
    let no_folder = harness
        .filing
        .add_meeting(&missing_folder, &id("one-off"))
        .await;

    assert!(matches!(no_meeting, Err(FolderError::MeetingNotFound(_))));
    assert!(matches!(no_folder, Err(FolderError::NotFound(_))));
}

#[tokio::test]
async fn taking_a_series_meeting_out_stops_collecting_the_series() {
    let harness = harness();
    let standups = harness.folder("Standups").await;
    harness
        .filing
        .add_meeting(&standups.id, &id("standup-1"))
        .await
        .unwrap();
    harness
        .filing
        .add_rule(&standups.id, series_rule())
        .await
        .unwrap();

    let removed = harness
        .filing
        .remove_meeting(&standups.id, &id("standup-1"))
        .await
        .unwrap();
    let again = harness
        .filing
        .remove_meeting(&standups.id, &id("standup-1"))
        .await
        .unwrap();

    assert!(removed.series_rule_removed);
    assert!(!again.series_rule_removed);
    assert!(harness.filing.rules(&standups.id).await.unwrap().is_empty());
    assert!(
        harness
            .filing
            .meetings(&standups.id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn adding_a_rule_twice_returns_the_first_and_domains_are_normalised() {
    let harness = harness();
    let clients = harness.folder("Clients").await;
    let domain = |text: &str| FolderRule::AttendeeDomain {
        domain: text.into(),
    };

    let first = harness
        .filing
        .add_rule(&clients.id, domain("@Acme.com"))
        .await
        .unwrap();
    let again = harness
        .filing
        .add_rule(&clients.id, domain("acme.com"))
        .await
        .unwrap();
    let invalid = harness.filing.add_rule(&clients.id, domain("acme")).await;

    assert_eq!(first.rule, domain("acme.com"));
    assert_eq!(again.id, first.id);
    assert!(matches!(invalid, Err(FolderError::InvalidDomain)));
    assert_eq!(harness.filing.rules(&clients.id).await.unwrap(), [first]);
}

#[tokio::test]
async fn removing_a_rule_that_is_gone_is_not_found() {
    let harness = harness();
    let clients = harness.folder("Clients").await;
    let rule = harness
        .filing
        .add_rule(&clients.id, series_rule())
        .await
        .unwrap();

    harness.filing.remove_rule(&rule.id).await.unwrap();
    let again = harness.filing.remove_rule(&rule.id).await;

    assert!(matches!(again, Err(FolderError::RuleNotFound(found)) if found == rule.id));
}

#[tokio::test]
async fn auto_filing_files_the_next_meeting_of_a_collected_series() {
    let harness = harness();
    let standups = harness.folder("Standups").await;
    harness
        .filing
        .add_rule(&standups.id, series_rule())
        .await
        .unwrap();

    let outcome = harness.auto.auto_file(&id("standup-2")).await.unwrap();
    let rerun = harness.auto.auto_file(&id("standup-2")).await.unwrap();

    assert_eq!(outcome, AutoFiled::Filed(vec![standups.id]));
    assert_eq!(rerun, outcome);
    assert_eq!(
        harness.filing.meetings(&standups.id).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn auto_filing_matches_tags_and_attendee_domains() {
    let harness = harness();
    let (customers, acme) = (
        harness.folder("Customers").await,
        harness.folder("Acme").await,
    );
    let tag = TagId::generate(&FakeClock::new(at(NOW)));
    harness.repo.set_facts(
        &id("one-off"),
        MeetingFacts {
            series_id: None,
            tag_ids: BTreeSet::from([tag]),
            attendee_domains: BTreeSet::from(["acme.com".to_owned()]),
        },
    );
    harness
        .filing
        .add_rule(&customers.id, FolderRule::Tag { tag_id: tag })
        .await
        .unwrap();
    let domain = FolderRule::AttendeeDomain {
        domain: "acme.com".into(),
    };
    harness.filing.add_rule(&acme.id, domain).await.unwrap();

    let outcome = harness.auto.auto_file(&id("one-off")).await.unwrap();

    let AutoFiled::Filed(folders) = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(folders.len(), 2);
    assert_eq!(
        harness
            .filing
            .folders_of(&id("one-off"))
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn auto_filing_says_why_it_filed_nothing() {
    let harness = harness();

    let no_rules = harness.auto.auto_file(&id("one-off")).await.unwrap();
    let standups = harness.folder("Standups").await;
    harness
        .filing
        .add_rule(&standups.id, series_rule())
        .await
        .unwrap();
    let no_match = harness.auto.auto_file(&id("one-off")).await.unwrap();
    let gone = harness.auto.auto_file(&id("meeting-x")).await.unwrap();

    assert_eq!(no_rules, AutoFiled::NoRules);
    assert_eq!(no_match, AutoFiled::NoMatch);
    assert_eq!(gone, AutoFiled::MeetingGone);
}
