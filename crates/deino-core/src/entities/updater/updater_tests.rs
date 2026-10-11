//! A processed meeting's people and companies catch up with it.

use deino_domain::MeetingId;
use deino_llm::{MockProvider, Route, mock_gateway};

use super::*;
use crate::companies::Company;
use crate::entities::testing::{NOW, World, time};
use crate::people::Person;
use crate::segments::SqliteSegmentRepo;

fn updater(world: &World, mock: Option<MockProvider>) -> EntityUpdateService {
    let entities = world.ports();
    let gateway = mock.map(|mock| Arc::new(mock_gateway(Arc::new(mock)).gateway));
    let summarizer = Arc::new(EntitySummarizer::new(entities.clone(), gateway));
    EntityUpdateService::new(EntityUpdatePorts {
        entities,
        speakers: Arc::new(SqliteSegmentRepo::new(world.database.clone())),
        summarizer,
        logos: None,
    })
}

/// Maya at Deino, with last week's briefing, and the user, in a new 1:1
/// whose summary says something new.
async fn new_one_on_one(world: &World) -> (MeetingId, Person, Company, Person) {
    let deino = world.company("Deino", Some("deino.dev")).await;
    let mut maya = world.person("Maya Chen", Some("maya@deino.dev")).await;
    world.employ(&mut maya, &deino).await;
    let me = world.person("Shreyas", Some("shreyas@deino.dev")).await;
    world.make_me(&me.id).await;
    let briefed = "UPDATE people SET summary_md = 'Old briefing',
        summary_updated_at = '2026-10-01T09:00:00Z' WHERE id = ?1";
    world.run(briefed, &[&maya.id.to_string()]).await;
    let meeting = world
        .meeting("new", "1:1 Maya", "2026-10-11T10:00:00Z")
        .await;
    world
        .attend(&meeting, &maya.id, AttendeeRole::Invitee)
        .await;
    world
        .attend(&meeting, &me.id, AttendeeRole::Organizer)
        .await;
    world
        .summarize(&meeting, "Maya takes over the sync rewrite.")
        .await;
    (meeting, maya, deino, me)
}

#[tokio::test]
async fn summaries_refresh_after_a_new_meeting_with_them() {
    let world = World::new().await;
    let (meeting, maya, deino, me) = new_one_on_one(&world).await;
    let mock = MockProvider::new()
        .reply(Route::EntitySummary, "Maya now leads the sync rewrite.")
        .reply(Route::EntitySummary, "Deino: the user's own team.");

    let update = updater(&world, Some(mock))
        .update_entities(&meeting)
        .await
        .unwrap();

    assert_eq!(update.summaries, 2, "Maya and her company, never the user");
    let stored = world.stored_person(&maya.id).await;
    assert_eq!(
        stored.summary_md.as_deref(),
        Some("Maya now leads the sync rewrite.")
    );
    assert_eq!(stored.summary_updated_at, Some(time(NOW)));
    let company = world.stored_company(&deino.id).await;
    assert_eq!(
        company.summary_md.as_deref(),
        Some("Deino: the user's own team.")
    );
    assert_eq!(world.stored_person(&me.id).await.summary_md, None);
}

/// Speakers named after `people` in meeting `demo`.
async fn named_speakers(world: &World, people: &[&Person]) {
    for (n, person) in people.iter().enumerate() {
        let id = format!("01929999-0000-7000-8000-0000000000b{n}");
        let label = format!("Speaker {}", n + 1);
        world
            .run(
                "INSERT INTO speakers (id, meeting_id, label, person_id, color)
                 VALUES (?1, 'demo', ?2, ?3, 1)",
                &[&id, &label, &person.id.to_string()],
            )
            .await;
    }
}

#[tokio::test]
async fn named_speakers_who_were_not_invited_join_the_meeting() {
    let world = World::new().await;
    let guest = world.person("Dana Ortiz", None).await;
    let invited = world.person("Hannah Pike", None).await;
    let meeting = world.meeting("demo", "Demo", "2026-10-11T10:00:00Z").await;
    world
        .attend(&meeting, &invited.id, AttendeeRole::Invitee)
        .await;
    named_speakers(&world, &[&invited, &guest]).await;

    let update = updater(&world, None)
        .update_entities(&meeting)
        .await
        .unwrap();

    assert_eq!(
        update,
        EntityUpdate {
            speakers_linked: 1,
            summaries: 0
        }
    );
    let sql = "SELECT role FROM meeting_attendees WHERE person_id = ?1";
    let role = world.text(sql, &[&guest.id.to_string()]).await;
    assert_eq!(role.as_deref(), Some("speaker_only"));
}

#[tokio::test]
async fn a_failing_model_call_fails_the_update_so_the_stage_can_retry() {
    let world = World::new().await;
    let (meeting, ..) = new_one_on_one(&world).await;

    let result = updater(&world, Some(MockProvider::new()))
        .update_entities(&meeting)
        .await;

    assert!(matches!(result, Err(EntityError::Llm(_))));
}

#[tokio::test]
async fn without_a_model_speakers_still_link_and_nothing_is_rewritten() {
    let world = World::new().await;
    let (meeting, maya, ..) = new_one_on_one(&world).await;

    let update = updater(&world, None)
        .update_entities(&meeting)
        .await
        .unwrap();

    assert_eq!(update.summaries, 0);
    let stored = world.stored_person(&maya.id).await;
    assert_eq!(stored.summary_md.as_deref(), Some("Old briefing"));
}
