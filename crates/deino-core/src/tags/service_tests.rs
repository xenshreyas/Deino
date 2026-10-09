//! [`TagService`] over the fake ports: CRUD rules, applying with a source,
//! and `MeetingTagged`.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{Clock, FakeClock, MeetingId, PaletteColor, TagId, TagSource};
use deino_testkit::at;

use super::*;
use crate::meetings::Meeting;

const NOW: &str = "2026-10-09T09:00:00Z";

struct Harness {
    tags: TagService,
    events: Arc<RecordedTagEvents>,
    clock: Arc<FakeClock>,
}

fn harness() -> Harness {
    let meeting = Meeting {
        id: MeetingId::new("meeting-1"),
        title: "Standup".into(),
        created_at: at(NOW),
        updated_at: at(NOW),
    };
    let repo = Arc::new(FakeTagRepo::with_meetings([meeting]));
    let events = Arc::new(RecordedTagEvents::default());
    let clock = Arc::new(FakeClock::new(at(NOW)));
    let tags = TagService::new(TagPorts {
        tags: repo.clone(),
        applied: repo,
        events: events.clone(),
        clock: clock.clone(),
    });
    Harness {
        tags,
        events,
        clock,
    }
}

fn new_tag(name: &str) -> NewTag {
    NewTag {
        name: name.into(),
        color: None,
        description: " For customer calls ".into(),
    }
}

fn meeting() -> MeetingId {
    MeetingId::new("meeting-1")
}

#[tokio::test]
async fn create_trims_picks_the_next_colour_and_lets_the_ai_apply_it() {
    let harness = harness();

    let first = harness.tags.create(&new_tag(" Customer ")).await.unwrap();
    let second = harness.tags.create(&new_tag("Hiring")).await.unwrap();
    let chosen = NewTag {
        color: Some(PaletteColor::new(5).unwrap()),
        ..new_tag("Board")
    };
    let third = harness.tags.create(&chosen).await.unwrap();

    assert_eq!(
        (first.name.as_str(), first.description.as_str()),
        ("Customer", "For customer calls")
    );
    assert_eq!(
        [first.color.slot(), second.color.slot(), third.color.slot()],
        [1, 2, 5]
    );
    assert!(first.auto_apply);
    let names: Vec<String> = harness
        .tags
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(names, ["Board", "Customer", "Hiring"]);
}

#[tokio::test]
async fn names_are_unique_ignoring_case_on_create_and_update() {
    let harness = harness();
    let customer = harness.tags.create(&new_tag("Customer")).await.unwrap();
    let hiring = harness.tags.create(&new_tag("Hiring")).await.unwrap();
    let rename = |name: &str| TagUpdate {
        name: name.into(),
        color: hiring.color,
        description: String::new(),
        auto_apply: false,
    };

    let clash = harness.tags.create(&new_tag("customer")).await;
    let taken = harness.tags.update(&hiring.id, &rename("CUSTOMER")).await;
    let recased = harness
        .tags
        .update(
            &customer.id,
            &TagUpdate {
                name: "customer".into(),
                ..rename("")
            },
        )
        .await;

    assert!(matches!(clash, Err(TagError::NameTaken)));
    assert!(matches!(taken, Err(TagError::NameTaken)));
    assert_eq!(recased.unwrap().name, "customer");
}

#[tokio::test]
async fn update_replaces_every_editable_field_and_stamps_the_clock() {
    let harness = harness();
    let tag = harness.tags.create(&new_tag("Customer")).await.unwrap();
    harness.clock.advance(TimeDelta::minutes(5));
    let update = TagUpdate {
        name: "Customers".into(),
        color: PaletteColor::new(4).unwrap(),
        description: "Paying customers".into(),
        auto_apply: false,
    };

    let updated = harness.tags.update(&tag.id, &update).await.unwrap();

    assert_eq!(updated.name, "Customers");
    assert_eq!(updated.color.slot(), 4);
    assert!(!updated.auto_apply);
    assert_eq!(
        (updated.created_at, updated.updated_at),
        (at(NOW), harness.clock.now())
    );
    assert_eq!(harness.tags.list().await.unwrap(), [updated]);
}

#[tokio::test]
async fn invalid_input_and_missing_tags_are_refused() {
    let harness = harness();
    let missing = TagId::generate(harness.clock.as_ref());

    let blank = harness.tags.create(&new_tag("  ")).await;
    let long = harness
        .tags
        .create(&NewTag {
            description: "x".repeat(501),
            ..new_tag("A")
        })
        .await;
    let gone = harness.tags.delete(&missing).await;
    let unknown = harness
        .tags
        .apply(&meeting(), &missing, TagSource::User)
        .await;

    assert!(matches!(blank, Err(TagError::EmptyName)));
    assert!(matches!(long, Err(TagError::DescriptionTooLong)));
    assert!(matches!(gone, Err(TagError::NotFound(_))));
    assert!(matches!(unknown, Err(TagError::NotFound(_))));
}

#[tokio::test]
async fn applying_records_the_source_and_announces_only_new_tags() {
    let harness = harness();
    let tag = harness.tags.create(&new_tag("Customer")).await.unwrap();

    let first = harness
        .tags
        .apply(&meeting(), &tag.id, TagSource::User)
        .await
        .unwrap();
    let again = harness
        .tags
        .apply(&meeting(), &tag.id, TagSource::Ai)
        .await
        .unwrap();

    assert!(first);
    assert!(!again);
    assert_eq!(
        harness.events.tagged(),
        [MeetingTagged {
            meeting_id: meeting(),
            tag_id: tag.id,
            applied_by: TagSource::User
        }]
    );
    let applied = harness.tags.tags_of(&meeting()).await.unwrap();
    assert_eq!(applied[0].applied_by, TagSource::User);
    assert_eq!(harness.tags.links().await.unwrap().len(), 1);
}

#[tokio::test]
async fn removing_and_deleting_take_tags_off_meetings() {
    let harness = harness();
    let (customer, hiring) = (
        harness.tags.create(&new_tag("Customer")).await.unwrap(),
        harness.tags.create(&new_tag("Hiring")).await.unwrap(),
    );
    harness
        .tags
        .apply(&meeting(), &customer.id, TagSource::User)
        .await
        .unwrap();
    harness
        .tags
        .apply(&meeting(), &hiring.id, TagSource::Ai)
        .await
        .unwrap();

    harness.tags.remove(&meeting(), &customer.id).await.unwrap();
    harness.tags.remove(&meeting(), &customer.id).await.unwrap();
    harness.tags.delete(&hiring.id).await.unwrap();

    assert!(harness.tags.tags_of(&meeting()).await.unwrap().is_empty());
    assert!(harness.tags.links().await.unwrap().is_empty());
}
