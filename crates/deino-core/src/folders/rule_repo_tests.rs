//! The rule and facts ports' contract against SQLite and the fake, what
//! only the schema can check, and auto-filing a seeded fixture series.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_db::{Database, test_db};
use deino_domain::{
    FakeClock, FolderId, FolderRuleId, MeetingId, MeetingSource, MeetingStatus, SeriesId,
};
use deino_testkit::fixture::{Fixtures, MeetingKind, seed_fixtures};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::{
    MeetingLifecycleRepo, MeetingLink, MeetingTitle, NewMeeting, SqliteMeetingRepo,
};
use crate::tags::{SqliteTagRepo, Tag, TagApplication, TagRepo};
use crate::testing::query_plan;

const NOW: &str = "2026-10-09T09:00:00Z";

fn clock() -> FakeClock {
    FakeClock::new(at(NOW))
}

fn folder(name: &str) -> Folder {
    Folder {
        id: FolderId::generate(&clock()),
        parent_id: None,
        name: name.into(),
        icon: None,
        description: String::new(),
        created_at: at(NOW),
        updated_at: at(NOW),
    }
}

fn new_rule(folder: &Folder, rule: FolderRule, minutes: i64) -> NewFolderRule {
    NewFolderRule {
        id: FolderRuleId::generate(&clock()),
        folder_id: folder.id,
        rule,
        created_at: at(NOW) + TimeDelta::minutes(minutes),
    }
}

fn series(id: &str) -> FolderRule {
    FolderRule::Series {
        series_id: SeriesId::new(id),
    }
}

/// Rules list per folder in the order added; a repeat changes nothing.
async fn rules_list_in_order_and_repeat_once(repo: &(impl FolderRepo + FolderRuleRepo)) {
    let (work, home) = (folder("Work"), folder("Home"));
    repo.create_folder(&work).await.unwrap();
    repo.create_folder(&home).await.unwrap();
    let domain = FolderRule::AttendeeDomain {
        domain: "acme.com".into(),
    };

    assert!(
        repo.add_rule(&new_rule(&work, domain.clone(), 2))
            .await
            .unwrap()
    );
    assert!(
        repo.add_rule(&new_rule(&work, series("s-1"), 1))
            .await
            .unwrap()
    );
    assert!(
        repo.add_rule(&new_rule(&home, series("s-1"), 0))
            .await
            .unwrap()
    );
    assert!(
        !repo
            .add_rule(&new_rule(&work, domain.clone(), 3))
            .await
            .unwrap()
    );

    let of_work: Vec<FolderRule> = repo
        .rules_of(&work.id)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.rule)
        .collect();
    assert_eq!(of_work, [series("s-1"), domain]);
    assert_eq!(repo.rules().await.unwrap().len(), 3);
}

/// Removing a series rule touches only that folder's rule.
async fn series_rules_are_removed_per_folder(repo: &(impl FolderRepo + FolderRuleRepo)) {
    let (work, home) = (folder("Work"), folder("Home"));
    repo.create_folder(&work).await.unwrap();
    repo.create_folder(&home).await.unwrap();
    let kept = new_rule(&home, series("s-1"), 0);
    repo.add_rule(&new_rule(&work, series("s-1"), 0))
        .await
        .unwrap();
    repo.add_rule(&kept).await.unwrap();

    assert!(
        repo.remove_series_rule(&work.id, &SeriesId::new("s-1"))
            .await
            .unwrap()
    );
    assert!(
        !repo
            .remove_series_rule(&work.id, &SeriesId::new("s-1"))
            .await
            .unwrap()
    );
    assert!(repo.remove_rule(&kept.id).await.unwrap());
    assert!(repo.rules().await.unwrap().is_empty());
}

/// Deleting a folder deletes its rules.
async fn deleting_a_folder_deletes_its_rules(repo: &(impl FolderRepo + FolderRuleRepo)) {
    let work = folder("Work");
    repo.create_folder(&work).await.unwrap();
    repo.add_rule(&new_rule(&work, series("s-1"), 0))
        .await
        .unwrap();

    repo.delete_folder(&work.id).await.unwrap();

    assert!(repo.rules().await.unwrap().is_empty());
}

async fn sqlite_repo() -> SqliteFolderRepo {
    SqliteFolderRepo::new(test_db().await)
}

#[tokio::test]
async fn sqlite_rules_list_in_order_and_repeat_once() {
    rules_list_in_order_and_repeat_once(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_rules_list_in_order_and_repeat_once() {
    rules_list_in_order_and_repeat_once(&FakeFolderRepo::default()).await;
}

#[tokio::test]
async fn sqlite_series_rules_are_removed_per_folder() {
    series_rules_are_removed_per_folder(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_series_rules_are_removed_per_folder() {
    series_rules_are_removed_per_folder(&FakeFolderRepo::default()).await;
}

#[tokio::test]
async fn sqlite_deleting_a_folder_deletes_its_rules() {
    deleting_a_folder_deletes_its_rules(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_deleting_a_folder_deletes_its_rules() {
    deleting_a_folder_deletes_its_rules(&FakeFolderRepo::default()).await;
}

async fn tag(database: &Database, name: &str) -> Tag {
    let tag = Tag {
        id: deino_domain::TagId::generate(&clock()),
        name: name.into(),
        color: deino_domain::PaletteColor::cycle(0),
        description: String::new(),
        auto_apply: true,
        created_at: at(NOW),
        updated_at: at(NOW),
    };
    SqliteTagRepo::new(database.clone())
        .create_tag(&tag)
        .await
        .unwrap();
    tag
}

#[tokio::test]
async fn deleting_a_tag_deletes_the_rules_that_name_it() {
    let repo = sqlite_repo().await;
    let work = folder("Work");
    repo.create_folder(&work).await.unwrap();
    let customer = tag(&repo.database, "Customer").await;
    let rule = FolderRule::Tag {
        tag_id: customer.id,
    };
    repo.add_rule(&new_rule(&work, rule, 0)).await.unwrap();

    SqliteTagRepo::new(repo.database.clone())
        .delete_tag(&customer.id)
        .await
        .unwrap();

    assert!(repo.rules_of(&work.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_rule_naming_a_missing_tag_is_a_foreign_key_violation() {
    let repo = sqlite_repo().await;
    let work = folder("Work");
    repo.create_folder(&work).await.unwrap();
    let ghost = FolderRule::Tag {
        tag_id: deino_domain::TagId::generate(&clock()),
    };

    let error = repo.add_rule(&new_rule(&work, ghost, 0)).await.unwrap_err();

    assert!(error.is_foreign_key_violation());
}

#[tokio::test]
async fn series_rules_carry_the_title_of_the_latest_meeting_in_the_series() {
    let (database, fixtures) = seeded().await;
    let standup = fixtures.meeting(MeetingKind::Standup).unwrap();
    let series_id = series_of(&database, &standup.id).await;
    let repo = SqliteFolderRepo::new(database);
    let work = folder("Work");
    repo.create_folder(&work).await.unwrap();
    repo.add_rule(&new_rule(&work, FolderRule::Series { series_id }, 0))
        .await
        .unwrap();
    repo.add_rule(&new_rule(&work, series("no-meetings"), 1))
        .await
        .unwrap();

    let titles: Vec<Option<String>> = repo
        .rules_of(&work.id)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.series_title)
        .collect();

    assert_eq!(titles, [Some(standup.title.clone()), None]);
}

#[tokio::test]
async fn facts_carry_a_meetings_series_tags_and_attendee_domains() {
    let (database, fixtures) = seeded().await;
    let sales = fixtures.meeting(MeetingKind::SalesDiscovery).unwrap();
    let customer = tag(&database, "Customer").await;
    let application = TagApplication {
        meeting_id: sales.id.clone(),
        tag_id: customer.id,
        applied_by: deino_domain::TagSource::User,
        applied_at: at(NOW),
    };
    crate::tags::MeetingTagRepo::apply_tag(&SqliteTagRepo::new(database.clone()), &application)
        .await
        .unwrap();
    let repo = SqliteFolderRepo::new(database);

    let facts = repo.facts(&sales.id).await.unwrap().unwrap();

    assert!(facts.tag_ids.contains(&customer.id));
    let expected: std::collections::BTreeSet<String> = sales
        .attendees
        .iter()
        .map(|attendee| attendee.email.domain().to_owned())
        .collect();
    assert_eq!(facts.attendee_domains, expected);
    assert!(
        repo.facts(&MeetingId::new("meeting-x"))
            .await
            .unwrap()
            .is_none()
    );
}

async fn seeded() -> (Database, Fixtures) {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    (database, Fixtures::load().unwrap())
}

async fn series_of(database: &Database, meeting: &MeetingId) -> SeriesId {
    let state = SqliteMeetingRepo::new(database.clone())
        .state(meeting)
        .await
        .unwrap()
        .unwrap();
    state
        .series_id
        .expect("the standup fixture is a recurring series")
}

/// ORG-05 acceptance: a recurring rule files the next fixture instance.
#[tokio::test]
async fn a_series_rule_files_the_next_instance_of_a_fixture_series() {
    let (database, fixtures) = seeded().await;
    let standup = fixtures.meeting(MeetingKind::Standup).unwrap();
    let repo = Arc::new(SqliteFolderRepo::new(database.clone()));
    let ports = FolderPorts::over(repo.clone(), Arc::new(clock()));
    let standups = FolderService::new(&ports)
        .create(&NewFolder {
            name: "Standups".into(),
            parent_id: None,
        })
        .await
        .unwrap();
    let filing = FilingService::new(&ports);
    let offer = filing
        .add_meeting(&standups.id, &standup.id)
        .await
        .unwrap()
        .series_offer;
    filing
        .add_rule(
            &standups.id,
            FolderRule::Series {
                series_id: offer.unwrap(),
            },
        )
        .await
        .unwrap();
    let next = next_instance(&database, &standup.id).await;

    let outcome = AutoFileService::new(&ports).auto_file(&next).await.unwrap();

    assert_eq!(outcome, AutoFiled::Filed(vec![standups.id]));
    let filed: Vec<MeetingId> = filing
        .meetings(&standups.id)
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect();
    assert_eq!(filed, [next, standup.id.clone()]);
}

/// Records tomorrow's meeting of `meeting`'s series.
async fn next_instance(database: &Database, meeting: &MeetingId) -> MeetingId {
    let meetings = SqliteMeetingRepo::new(database.clone());
    let series_id = series_of(database, meeting).await;
    let next = MeetingId::new("meeting-standup-next");
    let tomorrow = at("2026-10-09T15:30:00Z");
    let new = NewMeeting {
        id: next.clone(),
        title: MeetingTitle::parse("Eng standup").unwrap(),
        status: MeetingStatus::Processing,
        source: MeetingSource::Live,
        created_at: tomorrow,
        started_at: Some(tomorrow),
        folder_path: None,
    };
    meetings.create(&new).await.unwrap();
    let link = MeetingLink {
        calendar_event_id: None,
        series_id: Some(series_id),
    };
    meetings.link(&next, &link, tomorrow).await.unwrap();
    next
}

#[tokio::test]
async fn rule_and_filing_queries_read_through_their_indexes() {
    let repo = sqlite_repo().await;
    seed::meeting(&repo.database, "meeting-1", "Standup", at(NOW)).await;
    let plan = |sql| query_plan(&repo.database, sql);

    assert_eq!(
        plan(include_str!("queries/rules_of.sql")).await,
        [
            "SEARCH r USING INDEX idx_folder_rules_folder_id (folder_id=?)",
            "CORRELATED SCALAR SUBQUERY 1",
            "SEARCH m USING INDEX idx_meetings_series_id (series_id=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
    assert_eq!(
        plan(include_str!("queries/rules.sql")).await,
        ["SCAN folder_rules USING INDEX idx_folder_rules_folder_id"]
    );
    assert_eq!(
        plan(include_str!("queries/unfiled.sql")).await,
        [
            "SCAN m USING INDEX idx_meetings_live_created_at_id",
            "CORRELATED SCALAR SUBQUERY 1",
            "SEARCH mf USING COVERING INDEX sqlite_autoindex_meeting_folders_1 (meeting_id=?)"
        ]
    );
    assert_eq!(
        plan(include_str!("queries/meeting_counts.sql")).await,
        [
            "SCAN mf USING COVERING INDEX idx_meeting_folders_folder_id",
            "SEARCH m USING INDEX sqlite_autoindex_meetings_1 (id=?)"
        ]
    );
}
