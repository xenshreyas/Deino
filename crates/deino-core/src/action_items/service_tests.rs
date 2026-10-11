//! The service over the real repositories: every filter, the "Today"
//! digest, inline and bulk edits, deletes and "Mark all mine done".

use std::sync::Arc;

use chrono::{NaiveDate, TimeDelta};
use deino_domain::{
    ActionItemOrigin, ActionItemPriority, ActionItemStatus, ErrorClass, ErrorCode, MeetingId,
};

use crate::action_items::test_world::{ACME_CALL, STANDUP, Spec, TRASHED, World};
use crate::action_items::*;

fn titles(items: &[ActionItemView]) -> Vec<&str> {
    items.iter().map(|item| item.title.as_str()).collect()
}

/// Mine: overdue, today, Sunday, next week, undated, done; Maya's; Owen's
/// on the customer call; an unassigned one; one on a trashed meeting.
async fn populated() -> World {
    let world = World::new().await;
    let (me, maya, owen) = (Some(world.me), Some(world.maya), Some(world.owen));
    let rows = [
        ("Overdue deck", Some(STANDUP), me, Some("2026-10-08")),
        ("Today's review", Some(STANDUP), me, Some("2026-10-09")),
        ("Sunday prep", None, me, Some("2026-10-11")),
        ("Next week plan", None, me, Some("2026-10-12")),
        ("Someday idea", None, me, None),
        ("Maya's endpoint", Some(STANDUP), maya, Some("2026-10-09")),
        ("Owen's export", Some(ACME_CALL), owen, None),
        ("Nobody's task", Some(ACME_CALL), None, None),
        ("Trashed task", Some(TRASHED), me, None),
    ];
    for (title, meeting, assignee, due) in rows {
        world
            .item(Spec {
                title,
                meeting,
                assignee,
                due,
                done: false,
            })
            .await;
    }
    let done = Spec {
        assignee: me,
        due: Some("2026-10-01"),
        done: true,
        ..Spec::titled("Done already")
    };
    world.item(done).await;
    world
}

/// The edit that ticks or unticks an item.
fn tick(status: ActionItemStatus) -> ActionItemEdit {
    ActionItemEdit::Status { status }
}

/// `titles` in alphabetical order, for items whose list order is a tie.
fn sorted(mut titles: Vec<String>) -> Vec<String> {
    titles.sort();
    titles
}

async fn list(world: &World, filter: ActionItemFilter) -> Vec<String> {
    let (service, _) = world.service();
    let items = service.list(&filter).await.unwrap();
    titles(&items).into_iter().map(str::to_owned).collect()
}

#[tokio::test]
async fn the_default_view_is_my_open_items_by_due_date_without_trashed_meetings() {
    let world = populated().await;

    assert_eq!(
        list(&world, ActionItemFilter::default()).await,
        [
            "Overdue deck",
            "Today's review",
            "Sunday prep",
            "Next week plan",
            "Someday idea"
        ]
    );
}

#[tokio::test]
async fn assignee_scopes_pick_others_one_person_or_nobody() {
    let world = populated().await;
    let scoped = |assignee| ActionItemFilter {
        assignee,
        ..ActionItemFilter::default()
    };

    assert_eq!(
        list(&world, scoped(AssigneeScope::Others)).await,
        ["Maya's endpoint", "Owen's export"]
    );
    assert_eq!(
        list(
            &world,
            scoped(AssigneeScope::Person {
                person_id: world.owen
            })
        )
        .await,
        ["Owen's export"]
    );
    assert_eq!(
        list(&world, scoped(AssigneeScope::Unassigned)).await,
        ["Nobody's task"]
    );
    assert_eq!(list(&world, scoped(AssigneeScope::Anyone)).await.len(), 8);
}

#[tokio::test]
async fn due_windows_follow_the_local_week() {
    let world = populated().await;
    let due = |due| ActionItemFilter {
        due: Some(due),
        ..ActionItemFilter::default()
    };

    assert_eq!(
        list(&world, due(DueFilter::Overdue)).await,
        ["Overdue deck"]
    );
    assert_eq!(
        list(&world, due(DueFilter::Today)).await,
        ["Today's review"]
    );
    assert_eq!(
        list(&world, due(DueFilter::ThisWeek)).await,
        ["Today's review", "Sunday prep"]
    );
    assert_eq!(list(&world, due(DueFilter::NoDate)).await, ["Someday idea"]);
}

#[tokio::test]
async fn statuses_show_done_items_on_request() {
    let world = populated().await;
    let status = |status| ActionItemFilter {
        status,
        ..ActionItemFilter::default()
    };

    assert_eq!(
        list(&world, status(StatusFilter::Done)).await,
        ["Done already"]
    );
    assert_eq!(list(&world, status(StatusFilter::Any)).await.len(), 6);
}

#[tokio::test]
async fn tag_company_and_folder_filters_follow_the_meeting() {
    let world = populated().await;
    let anyone = ActionItemFilter {
        assignee: AssigneeScope::Anyone,
        ..ActionItemFilter::default()
    };
    // Both are undated, so their order is the IDs'; compare them sorted.
    let customer_call = ["Nobody's task", "Owen's export"];

    let tagged = ActionItemFilter {
        tag_id: Some(world.customer_tag),
        ..anyone.clone()
    };
    let company = ActionItemFilter {
        company_id: Some(world.acme),
        ..anyone.clone()
    };
    // The call is filed in Clients › Acme; filtering by Clients includes it.
    let folder = ActionItemFilter {
        folder_id: Some(world.clients),
        ..anyone.clone()
    };

    assert_eq!(sorted(list(&world, tagged).await), customer_call);
    assert_eq!(sorted(list(&world, company).await), customer_call);
    assert_eq!(sorted(list(&world, folder).await), customer_call);
}

#[tokio::test]
async fn a_date_range_matches_the_meeting_day_or_a_standalone_items_creation() {
    let world = populated().await;
    let range = |from: &str, to: &str| ActionItemFilter {
        date_range: Some(DateRange {
            from: from.parse().ok(),
            to: to.parse().ok(),
        }),
        ..ActionItemFilter::default()
    };

    // The standup was saved on Thursday 8 October; standalone items today.
    assert_eq!(
        list(&world, range("2026-10-08", "2026-10-08")).await,
        ["Overdue deck", "Today's review"]
    );
    assert_eq!(
        list(&world, range("2026-10-09", "")).await,
        ["Sunday prep", "Next week plan", "Someday idea"]
    );
}

#[tokio::test]
async fn the_meeting_list_shows_every_item_of_the_meeting() {
    let world = populated().await;

    let mut titles = list(&world, ActionItemFilter::meeting(MeetingId::new(STANDUP))).await;
    // Two are due the same day; their order is the IDs'.
    titles[1..].sort();

    assert_eq!(
        titles,
        ["Overdue deck", "Maya's endpoint", "Today's review"]
    );
}

#[tokio::test]
async fn the_today_digest_has_my_overdue_and_due_today_items() {
    let world = populated().await;
    let (service, _) = world.service();

    let digest = service.today().await.unwrap();

    assert_eq!(titles(&digest.overdue), ["Overdue deck"]);
    assert_eq!(titles(&digest.due_today), ["Today's review"]);
}

#[tokio::test]
async fn created_items_are_mine_to_edit_and_are_announced() {
    let world = World::new().await;
    let (service, events) = world.service();
    let new = NewActionItem {
        title: "  Book the offsite  ".into(),
        assignee_person_id: Some(world.me),
        due: NaiveDate::from_ymd_opt(2026, 10, 16),
        priority: Some(ActionItemPriority::High),
        ..NewActionItem::default()
    };

    let created = service.create(new).await.unwrap();

    assert_eq!(created.title, "Book the offsite");
    assert_eq!(created.created_by, ActionItemOrigin::User);
    assert_eq!(created.due_at, Some(world.due("2026-10-16")));
    assert!(created.assignee_is_me);
    assert_eq!(created.meeting_id, None);
    assert_eq!(events.changes()[0].kind, ActionItemChangeKind::Created);
}

#[tokio::test]
async fn creating_for_an_unknown_person_or_without_a_title_fails() {
    let world = World::new().await;
    let (service, _) = world.service();
    let stranger = deino_domain::PersonId::generate(world.clock.as_ref());

    let blank = service.create(NewActionItem::default()).await.unwrap_err();
    let unknown = service
        .create(NewActionItem {
            title: "x".into(),
            assignee_person_id: Some(stranger),
            ..NewActionItem::default()
        })
        .await
        .unwrap_err();

    assert_eq!(blank.code(), ErrorCode::Invalid);
    assert_eq!(unknown.code(), ErrorCode::NotFound);
}

#[tokio::test]
async fn an_edit_marks_the_item_touched_and_returns_its_row() {
    let world = World::new().await;
    let item = world
        .item(Spec {
            title: "Draft",
            meeting: Some(STANDUP),
            ..Spec::default()
        })
        .await;
    let (service, events) = world.service();
    world.clock.advance(TimeDelta::minutes(5));

    let edited = service
        .update(
            item.id,
            &ActionItemEdit::Assignee {
                person_id: Some(world.maya),
            },
        )
        .await
        .unwrap();

    assert_eq!(edited.assignee_name.as_deref(), Some("Maya Chen"));
    assert_eq!(edited.meeting_title.as_deref(), Some("Eng standup"));
    assert!(edited.updated_at > edited.created_at);
    assert_eq!(events.changes()[0].kind, ActionItemChangeKind::Updated);
}

#[tokio::test]
async fn ticking_completes_and_unticking_reopens() {
    let world = World::new().await;
    let item = world.item(Spec::titled("Ship")).await;
    let (service, events) = world.service();

    let done = service
        .update(item.id, &tick(ActionItemStatus::Done))
        .await
        .unwrap();
    let open = service
        .update(item.id, &tick(ActionItemStatus::Open))
        .await
        .unwrap();

    assert_eq!(
        (done.status, done.completed_at.is_some()),
        (ActionItemStatus::Done, true)
    );
    assert_eq!(
        (open.status, open.completed_at),
        (ActionItemStatus::Open, None)
    );
    let kinds: Vec<ActionItemChangeKind> =
        events.changes().iter().map(|change| change.kind).collect();
    assert_eq!(
        kinds,
        [
            ActionItemChangeKind::Completed,
            ActionItemChangeKind::Reopened
        ]
    );
}

#[tokio::test]
async fn bulk_edits_reassign_and_redate_many_items() {
    let world = World::new().await;
    let ids = [
        world.item(Spec::titled("One")).await.id,
        world.item(Spec::titled("Two")).await.id,
    ];
    let (service, _) = world.service();
    let owen = ActionItemEdit::Assignee {
        person_id: Some(world.owen),
    };
    let monday = ActionItemEdit::Due {
        date: NaiveDate::from_ymd_opt(2026, 10, 12),
    };

    let reassigned = service.bulk_update(&ids, &owen).await.unwrap();
    let dated = service.bulk_update(&ids, &monday).await.unwrap();

    assert_eq!((reassigned, dated), (2, 2));
    let scope = AssigneeScope::Person {
        person_id: world.owen,
    };
    let items = service
        .list(&ActionItemFilter {
            assignee: scope,
            ..ActionItemFilter::default()
        })
        .await
        .unwrap();
    assert_eq!(items.len(), 2);
    assert!(
        items
            .iter()
            .all(|item| item.due_at == Some(world.due("2026-10-12")))
    );
}

#[tokio::test]
async fn bulk_edits_never_rewrite_wording() {
    let world = World::new().await;
    let ids = [world.item(Spec::titled("One")).await.id];
    let (service, _) = world.service();

    let renamed = service
        .bulk_update(
            &ids,
            &ActionItemEdit::Title {
                title: "Same".into(),
            },
        )
        .await;

    assert_eq!(renamed.unwrap_err().code(), ErrorCode::Invalid);
}

#[tokio::test]
async fn deleting_and_mark_all_mine_done() {
    let world = populated().await;
    let (service, _) = world.service();
    let anyone = ActionItemFilter {
        assignee: AssigneeScope::Anyone,
        ..ActionItemFilter::default()
    };
    let theirs = service.list(&anyone).await.unwrap();
    let nobody: Vec<_> = theirs
        .iter()
        .filter(|item| item.assignee_person_id.is_none())
        .map(|item| item.id)
        .collect();

    let deleted = service.delete(&nobody).await.unwrap();
    let completed = service.mark_all_mine_done().await.unwrap();

    assert_eq!((deleted, completed), (1, 5));
    assert!(
        service
            .list(&ActionItemFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        list(&world, anyone).await,
        ["Maya's endpoint", "Owen's export"]
    );
}

#[tokio::test]
async fn filter_options_put_me_first() {
    let world = World::new().await;
    let (service, _) = world.service();

    let options = service.filter_options().await.unwrap();

    let names: Vec<&str> = options
        .people
        .iter()
        .map(|person| person.name.as_str())
        .collect();
    assert_eq!(names, ["Shreyas Rao", "Maya Chen", "Owen Hart"]);
    assert!(options.people[0].is_me);
    assert_eq!(
        (
            options.companies.len(),
            options.tags.len(),
            options.folders.len()
        ),
        (1, 1, 2)
    );
}

#[tokio::test]
async fn saved_views_keep_their_order_and_unique_names() {
    let world = World::new().await;
    let views = SavedViewService::new(Arc::new(world.repo.clone()), world.clock.clone());
    let overdue = ActionItemFilter {
        due: Some(DueFilter::Overdue),
        ..ActionItemFilter::default()
    };
    let input = |name: &str| SavedViewInput {
        name: name.into(),
        filter: overdue.clone(),
    };

    let first = views.create(input("Overdue")).await.unwrap();
    let second = views.create(input("Team")).await.unwrap();
    let clash = views.create(input("overdue")).await.unwrap_err();
    let renamed = views.update(second.id, input("Whole team")).await.unwrap();
    views.delete(first.id).await.unwrap();

    assert_eq!((first.position, second.position), (0, 1));
    assert_eq!(clash.code(), ErrorCode::Conflict);
    assert_eq!(renamed.filter, overdue);
    let left: Vec<String> = views
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|view| view.name)
        .collect();
    assert_eq!(left, ["Whole team"]);
    assert_eq!(
        views.delete(first.id).await.unwrap_err().code(),
        ErrorCode::NotFound
    );
}
