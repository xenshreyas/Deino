//! Summaries on the `entity.summary` route, with scripted model answers.

use deino_domain::AttendeeRole;
use deino_llm::{MockProvider, Route, mock_gateway};

use super::*;
use crate::entities::testing::{NOW, World, time};

fn summarizer(world: &World, mock: MockProvider) -> (EntitySummarizer, Arc<MockProvider>) {
    let mock = Arc::new(mock);
    let gateway = Arc::new(mock_gateway(mock.clone()).gateway);
    (EntitySummarizer::new(world.ports(), Some(gateway)), mock)
}

#[tokio::test]
async fn a_persons_briefing_reads_their_meetings_and_is_stored_cleaned() {
    let world = World::new().await;
    let maya = world.person("Maya Chen", Some("maya@deino.dev")).await;
    let one = world
        .meeting("one", "1:1 Maya", "2026-10-07T10:00:00Z")
        .await;
    world.attend(&one, &maya.id, AttendeeRole::Invitee).await;
    world
        .summarize(&one, "Maya will split the webhook queue.")
        .await;
    let answer = "<think>plan</think>\nMaya leads the webhook queue split.";
    let (summarizer, mock) = summarizer(
        &world,
        MockProvider::new().reply(Route::EntitySummary, answer),
    );

    let outcome = summarizer.summarize_person(&maya.id).await.unwrap();

    assert_eq!(outcome, SummaryOutcome::Refreshed);
    let stored = world.stored_person(&maya.id).await;
    assert_eq!(
        stored.summary_md.as_deref(),
        Some("Maya leads the webhook queue split.")
    );
    assert_eq!(stored.summary_updated_at, Some(time(NOW)));
    let request = &mock.requests()[0];
    assert!(
        request.messages[0]
            .text()
            .contains("Maya will split the webhook queue.")
    );
}

#[tokio::test]
async fn nobody_with_nothing_written_about_them_costs_a_model_call() {
    let world = World::new().await;
    let noor = world.person("Noor Haddad", None).await;
    let (summarizer, mock) = summarizer(&world, MockProvider::new());

    let outcome = summarizer.summarize_person(&noor.id).await.unwrap();

    assert_eq!(outcome, SummaryOutcome::NoContent);
    assert!(mock.requests().is_empty());
}

#[tokio::test]
async fn an_answer_that_is_only_reasoning_leaves_the_summary_alone() {
    let world = World::new().await;
    let noor = world.person("Noor Haddad", None).await;
    world
        .run(
            "UPDATE people SET notes_md = 'Designer', summary_md = 'Kept'",
            &[],
        )
        .await;
    let mock = MockProvider::new().reply(Route::EntitySummary, "<think>unfinished");
    let (summarizer, _) = summarizer(&world, mock);

    let outcome = summarizer.summarize_person(&noor.id).await.unwrap();

    assert_eq!(outcome, SummaryOutcome::EmptyAnswer);
    assert_eq!(
        world.stored_person(&noor.id).await.summary_md.as_deref(),
        Some("Kept")
    );
}

#[tokio::test]
async fn a_companys_briefing_names_its_people_and_reads_their_meetings() {
    let world = World::new().await;
    let fernhollow = world.company("Fernhollow", Some("fernhollow.co")).await;
    let mut owen = world.person("Owen Price", None).await;
    world.employ(&mut owen, &fernhollow).await;
    let call = world
        .meeting("call", "Escalation", "2026-10-05T10:00:00Z")
        .await;
    world.attend(&call, &owen.id, AttendeeRole::Invitee).await;
    world.summarize(&call, "Duplicate jobs after v1.4.").await;
    let mock = MockProvider::new().reply(Route::EntitySummary, "A field-service customer.");
    let (summarizer, mock) = summarizer(&world, mock);

    summarizer.summarize_company(&fernhollow.id).await.unwrap();

    let stored = world.stored_company(&fernhollow.id).await;
    assert_eq!(
        stored.summary_md.as_deref(),
        Some("A field-service customer.")
    );
    let user = mock.requests()[0].messages[0].text();
    assert!(
        user.contains("People the user meets there: Owen Price"),
        "{user}"
    );
    assert!(user.contains("Website: fernhollow.co"), "{user}");
}

#[tokio::test]
async fn without_a_model_nothing_is_asked_or_stored() {
    let world = World::new().await;
    let noor = world.person("Noor Haddad", None).await;
    world
        .run("UPDATE people SET notes_md = 'Designer'", &[])
        .await;

    let outcome = EntitySummarizer::new(world.ports(), None)
        .summarize_person(&noor.id)
        .await;

    assert_eq!(outcome.unwrap(), SummaryOutcome::NoModel);
}
