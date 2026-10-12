//! [`MeetingListService`]: paging, ⌘A, the audience rule and dressing rows.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::TimeDelta;
use deino_db::test_db;
use deino_domain::{Audience, Email, FixedZone, MeetingId};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::test_seed as rows;
use crate::meetings::{MeetingError, SqliteMeetingRepo};
use crate::profile::InternalDomains;

/// The user's addresses, fixed for a test.
struct Addresses(Vec<Email>);

#[async_trait]
impl OwnAddresses for Addresses {
    async fn internal_domains(&self) -> InternalDomains {
        InternalDomains::from_emails(&self.0)
    }
}

async fn service(count: u32, emails: &[&str]) -> (MeetingListService, SqliteMeetingRepo) {
    let database = test_db().await;
    for n in 0..count {
        let created_at = at("2026-10-01T09:00:00Z") + TimeDelta::days(i64::from(n));
        seed::meeting(&database, &format!("m{n}"), "Standup", created_at).await;
    }
    let repo = SqliteMeetingRepo::new(database);
    let emails = emails
        .iter()
        .map(|email| Email::parse(email).unwrap())
        .collect();
    let service = MeetingListService::new(MeetingListDeps {
        repo: Arc::new(repo.clone()),
        zone: Arc::new(FixedZone::utc()),
        addresses: Arc::new(Addresses(emails)),
    });
    (service, repo)
}

fn ids(page: &MeetingPage) -> Vec<&str> {
    page.meetings
        .iter()
        .map(|meeting| meeting.id.as_str())
        .collect()
}

#[tokio::test]
async fn pages_end_with_a_cursor_only_while_more_rows_follow() {
    let (service, _) = service(3, &[]).await;
    let filter = MeetingFilter::default();

    let first = service.page(&filter, None, 2).await.unwrap();
    let last = service.page(&filter, first.next.as_ref(), 2).await.unwrap();

    assert_eq!(ids(&first), ["m2", "m1"]);
    assert_eq!(
        first.next.as_ref().map(|cursor| cursor.id.as_str()),
        Some("m1")
    );
    assert_eq!(ids(&last), ["m0"]);
    assert_eq!(last.next, None);
}

#[tokio::test]
async fn a_page_size_of_zero_still_returns_a_row() {
    let (service, _) = service(2, &[]).await;

    let page = service
        .page(&MeetingFilter::default(), None, 0)
        .await
        .unwrap();

    assert_eq!(ids(&page), ["m1"]);
}

#[tokio::test]
async fn select_all_returns_every_match_beyond_the_loaded_page() {
    let (service, _) = service(4, &[]).await;
    let filter = MeetingFilter {
        date_range: Some(DateRange {
            from: Some("2026-10-02".parse().unwrap()),
            to: None,
        }),
        ..MeetingFilter::default()
    };

    let all = service.matching_ids(&filter).await.unwrap();

    assert_eq!(all, ["m3", "m2", "m1"].map(MeetingId::new));
}

#[tokio::test]
async fn the_audience_filter_needs_the_users_address() {
    let (unknown, _) = service(1, &[]).await;
    let (known, _) = service(1, &["me@deino.dev"]).await;
    let filter = MeetingFilter {
        audience: Some(Audience::Internal),
        ..MeetingFilter::default()
    };

    let refused = unknown.page(&filter, None, 10).await.unwrap_err();
    let answered = known.page(&filter, None, 10).await.unwrap();

    assert!(matches!(refused, MeetingError::AudienceUnknown));
    assert!(
        answered.meetings.is_empty(),
        "m0 has no attendees, so it is neither"
    );
}

#[tokio::test]
async fn rows_carry_their_attendees_and_tags() {
    let (service, repo) = service(2, &[]).await;
    let maya = rows::person(&repo.database, 1, None, None).await;
    rows::attend(&repo.database, "m1", &maya, "invitee").await;
    let tag = rows::tag(&repo.database, 20, "Customer").await;
    rows::tag_meeting(&repo.database, "m0", &tag).await;

    let page = service
        .page(&MeetingFilter::default(), None, 10)
        .await
        .unwrap();

    let names: Vec<&str> = page.meetings[0]
        .attendees
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(names, ["Person 1"]);
    assert!(page.meetings[0].tags.is_empty());
    assert_eq!(page.meetings[1].tags[0].name, "Customer");
}
