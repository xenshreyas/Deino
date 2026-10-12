//! [`MeetingListRepo`] against the real schema: pages, every filter, the
//! query plan, and the 2,000-meeting benchmark (ORG-07's acceptance).

use chrono::TimeDelta;
use deino_db::{Database, test_db};
use deino_domain::{Audience, CompanyId, FolderId, MeetingId, PersonId, TagId};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::SqliteMeetingRepo;
use crate::meetings::test_seed as rows;
use crate::testing::query_plan;

/// Meetings `m0`..`m{count-1}`, one a day from 1 October, `m0` oldest.
async fn repo_with(count: u32) -> SqliteMeetingRepo {
    let database = test_db().await;
    for n in 0..count {
        let created_at = at("2026-10-01T09:00:00Z") + TimeDelta::days(i64::from(n));
        seed::meeting(&database, &format!("m{n}"), "Standup", created_at).await;
    }
    SqliteMeetingRepo::new(database)
}

async fn listed(repo: &SqliteMeetingRepo, query: &ListQuery) -> Vec<String> {
    let rows = repo.page(query, None, RowLimit::All).await.unwrap();
    rows.into_iter()
        .map(|row| row.id.as_str().to_owned())
        .collect()
}

fn db(repo: &SqliteMeetingRepo) -> &Database {
    &repo.database
}

#[tokio::test]
async fn pages_run_newest_first_and_resume_after_the_cursor() {
    let repo = repo_with(5).await;
    let query = ListQuery::default();

    let first = repo.page(&query, None, RowLimit::At(2)).await.unwrap();
    let cursor = first.last().unwrap().cursor();
    let second = repo
        .page(&query, Some(&cursor), RowLimit::At(2))
        .await
        .unwrap();

    let ids = |rows: &[MeetingRow]| {
        rows.iter()
            .map(|row| row.id.as_str().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&first), ["m4", "m3"]);
    assert_eq!(ids(&second), ["m2", "m1"]);
}

#[tokio::test]
async fn trashed_meetings_are_never_listed() {
    let repo = repo_with(2).await;
    rows::set(
        db(&repo),
        "m1",
        ("deleted_at", Some("2026-10-05T09:00:00Z")),
    )
    .await;

    assert_eq!(listed(&repo, &ListQuery::default()).await, ["m0"]);
}

#[tokio::test]
async fn people_match_attendees_or_linked_speakers_and_companies_their_people() {
    let repo = repo_with(4).await;
    let acme = rows::company(db(&repo), 90, "acme.com").await;
    let maya = rows::person(db(&repo), 1, Some("maya@acme.com"), Some(&acme)).await;
    let lee = rows::person(db(&repo), 2, None, None).await;
    rows::attend(db(&repo), "m0", &maya, "invitee").await;
    rows::speaker(db(&repo), 10, "m1", ("Speaker 1", Some(&lee))).await;

    let by_people = ListQuery {
        people: vec![maya.parse::<PersonId>().unwrap(), lee.parse().unwrap()],
        ..ListQuery::default()
    };
    let by_company = ListQuery {
        companies: vec![acme.parse::<CompanyId>().unwrap()],
        ..ListQuery::default()
    };

    assert_eq!(listed(&repo, &by_people).await, ["m1", "m0"]);
    assert_eq!(listed(&repo, &by_company).await, ["m0"]);
}

#[tokio::test]
async fn tags_folders_with_subfolders_and_action_items_narrow_the_list() {
    let repo = repo_with(4).await;
    let customer = rows::tag(db(&repo), 20, "Customer").await;
    let parent = rows::folder(db(&repo), 30, None).await;
    let child = rows::folder(db(&repo), 31, Some(&parent)).await;
    rows::tag_meeting(db(&repo), "m0", &customer).await;
    rows::file(db(&repo), "m1", &child).await;
    rows::action_item(db(&repo), 40, "m2").await;

    let by_tag = ListQuery {
        tags: vec![customer.parse::<TagId>().unwrap()],
        ..ListQuery::default()
    };
    let by_folder = ListQuery {
        folders: vec![parent.parse::<FolderId>().unwrap()],
        ..ListQuery::default()
    };
    let with_items = ListQuery {
        has_action_items: true,
        ..ListQuery::default()
    };
    let both = ListQuery {
        has_action_items: true,
        ..by_tag.clone()
    };

    assert_eq!(listed(&repo, &by_tag).await, ["m0"]);
    assert_eq!(
        listed(&repo, &by_folder).await,
        ["m1"],
        "a subfolder's meetings count"
    );
    assert_eq!(listed(&repo, &with_items).await, ["m2"]);
    assert!(listed(&repo, &both).await.is_empty(), "conditions combine");
}

#[tokio::test]
async fn the_date_window_is_half_open() {
    let repo = repo_with(4).await;
    let query = ListQuery {
        held_from: Some(at("2026-10-02T00:00:00Z")),
        held_before: Some(at("2026-10-04T00:00:00Z")),
        ..ListQuery::default()
    };

    assert_eq!(listed(&repo, &query).await, ["m2", "m1"]);
}

#[tokio::test]
async fn internal_and_external_follow_the_users_domains() {
    let repo = repo_with(3).await;
    let me = rows::person(db(&repo), 1, Some("me@deino.dev"), None).await;
    rows::make_me(db(&repo), &me).await;
    let colleague = rows::person(db(&repo), 2, Some("lee@eng.deino.dev"), None).await;
    let client = rows::person(db(&repo), 3, Some("ana@acme.com"), None).await;
    rows::attend(db(&repo), "m0", &me, "organizer").await;
    rows::attend(db(&repo), "m0", &colleague, "invitee").await;
    rows::attend(db(&repo), "m1", &client, "invitee").await;
    let audience = |audience| AudienceQuery {
        audience: Some(audience),
        own: vec!["me@deino.dev".into()],
        domains: vec!["deino.dev".into()],
    };

    let internal = ListQuery {
        audience: audience(Audience::Internal),
        ..ListQuery::default()
    };
    let external = ListQuery {
        audience: audience(Audience::External),
        ..ListQuery::default()
    };

    assert_eq!(
        listed(&repo, &internal).await,
        ["m0"],
        "m2 has no attendee list"
    );
    assert_eq!(listed(&repo, &external).await, ["m1"]);
}

#[tokio::test]
async fn a_pages_avatars_put_the_user_last_and_tags_go_by_name() {
    let repo = repo_with(1).await;
    let me = rows::person(db(&repo), 1, Some("me@deino.dev"), None).await;
    rows::make_me(db(&repo), &me).await;
    let maya = rows::person(db(&repo), 2, None, None).await;
    rows::attend(db(&repo), "m0", &me, "organizer").await;
    rows::attend(db(&repo), "m0", &maya, "invitee").await;
    let (zeta, alpha) = (
        rows::tag(db(&repo), 20, "Zeta").await,
        rows::tag(db(&repo), 21, "alpha").await,
    );
    rows::tag_meeting(db(&repo), "m0", &zeta).await;
    rows::tag_meeting(db(&repo), "m0", &alpha).await;
    let page = [MeetingId::new("m0")];

    let people: Vec<bool> = repo
        .attendees(&page)
        .await
        .unwrap()
        .iter()
        .map(|row| row.attendee.is_me)
        .collect();
    let tags: Vec<String> = repo
        .tags(&page)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.tag.name)
        .collect();

    assert_eq!(people, [false, true]);
    assert_eq!(tags, ["alpha", "Zeta"]);
}

#[tokio::test]
async fn the_page_walks_the_live_index_without_sorting() {
    let repo = repo_with(0).await;

    let plan = query_plan(db(&repo), include_str!("queries/page.sql")).await;

    assert!(
        plan.iter()
            .any(|step| step == "SCAN m USING INDEX idx_meetings_live_created_at_id"),
        "{plan:?}"
    );
    assert!(
        !plan
            .iter()
            .any(|step| step.contains("TEMP B-TREE FOR ORDER BY")),
        "{plan:?}"
    );
}

/// ORG-07's acceptance: filtering 2,000 seeded meetings takes under 150 ms.
/// Run with `cargo test -p deino-core --release -- --ignored filtering_2000`.
#[tokio::test]
#[ignore = "benchmark: run on demand in release mode"]
#[allow(
    clippy::disallowed_methods,
    clippy::print_stderr,
    reason = "a benchmark measures wall-clock time and reports it"
)]
async fn filtering_2000_meetings_takes_under_150_ms() {
    let repo = repo_with(2_000).await;
    let tag = rows::tag(db(&repo), 20, "Customer").await;
    let person = rows::person(db(&repo), 1, Some("ana@acme.com"), None).await;
    for n in (0..2_000).step_by(3) {
        rows::tag_meeting(db(&repo), &format!("m{n}"), &tag).await;
        rows::attend(db(&repo), &format!("m{n}"), &person, "invitee").await;
    }
    let query = ListQuery {
        tags: vec![tag.parse().unwrap()],
        people: vec![person.parse().unwrap()],
        held_from: Some(at("2026-10-01T00:00:00Z")),
        ..ListQuery::default()
    };

    let started = std::time::Instant::now();
    let all = repo.page(&query, None, RowLimit::All).await.unwrap();
    let page = repo.page(&query, None, RowLimit::At(50)).await.unwrap();
    let elapsed = started.elapsed();
    eprintln!("filtering 2,000 meetings (all matches + first page): {elapsed:?}");

    assert_eq!((all.len(), page.len()), (667, 50));
    assert!(elapsed.as_millis() < 150, "took {elapsed:?}");
}
