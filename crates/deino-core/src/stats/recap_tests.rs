//! The year in review: when it opens, what the model reads, and that it is
//! generated once.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_llm::{MockProvider, Route, mock_gateway};
use deino_testkit::fixture::{USER_EMAIL, seed_fixtures};

use super::testing::World;
use super::*;

/// A phrase spoken in the standup fixture's transcript, never in a title.
const SPOKEN: &str = "webhook";

async fn world() -> World {
    let world = World::new(&[USER_EMAIL]).await;
    seed_fixtures(&world.database).await.unwrap();
    world
}

fn service(world: &World, mock: Option<Arc<MockProvider>>, window: RecapWindow) -> RecapService {
    let gateway = mock.map(|mock| Arc::new(mock_gateway(mock).gateway));
    RecapService::new(
        world.ports(),
        RecapParts {
            recaps: Arc::new(SqliteRecapRepo::new(world.database.clone())),
            gateway,
            window,
        },
    )
}

fn answering(text: &str) -> Arc<MockProvider> {
    Arc::new(MockProvider::new().reply(Route::Recap, text))
}

#[tokio::test]
async fn outside_december_the_recap_is_closed() {
    let world = world().await;
    let recap = service(&world, Some(answering("unused")), RecapWindow::December);

    let status = recap.status().await.unwrap();
    let error = recap.generate().await.unwrap_err();

    assert_eq!(
        status,
        RecapStatus {
            year: 2026,
            available: false,
            recap: None
        }
    );
    assert!(matches!(error, StatsError::RecapNotYet));
}

#[tokio::test]
async fn in_december_the_recap_opens() {
    let world = world().await;
    world.clock.advance(TimeDelta::days(60));
    let recap = service(&world, Some(answering("unused")), RecapWindow::December);

    assert!(recap.status().await.unwrap().available);
}

#[tokio::test]
async fn the_recap_reads_numbers_and_titles_only_and_is_kept() {
    let world = world().await;
    let mock = answering("<think>plan</think>\nA year of standups.\n\n### People\nMaya.");
    let recap = service(&world, Some(Arc::clone(&mock)), RecapWindow::Anytime);

    let first = recap.generate().await.unwrap();
    let again = recap.generate().await.unwrap();

    assert_eq!(first.year, 2026);
    assert_eq!(first.markdown, "A year of standups.\n\n### People\nMaya.");
    assert_eq!(again, first);
    assert_eq!(recap.status().await.unwrap().recap, Some(first));
    let requests = mock.requests();
    assert_eq!(requests.len(), 1, "generated once, then read back");
    let prompt = requests[0].messages[0].text();
    assert!(prompt.contains("Meetings: 8"));
    assert!(prompt.contains("Hours recorded: 4"));
    assert!(prompt.contains("Met most: Priya Nair (4), Tom Becker (4)"));
    assert!(prompt.contains("Busiest weekday: Monday (2 meetings)"));
    assert!(prompt.contains("2026-10-08 Eng standup"));
    assert!(!prompt.to_lowercase().contains(SPOKEN));
}

#[tokio::test]
async fn without_a_model_there_is_no_recap() {
    let world = world().await;
    let recap = service(&world, None, RecapWindow::Anytime);

    let error = recap.generate().await.unwrap_err();

    assert!(matches!(error, StatsError::NoModel));
    assert_eq!(recap.status().await.unwrap().recap, None);
}

#[tokio::test]
async fn an_empty_answer_is_not_kept() {
    let world = world().await;
    let recap = service(&world, Some(answering("<think>")), RecapWindow::Anytime);

    let error = recap.generate().await.unwrap_err();

    assert!(matches!(error, StatsError::EmptyAnswer));
    assert_eq!(recap.status().await.unwrap().recap, None);
}
