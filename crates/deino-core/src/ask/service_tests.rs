//! [`AskService`] over the seeded fixtures, the SQLite stores and a mock
//! LLM: real citations, redaction, recipes that chain, preview, pruning.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use deino_db::{Database, test_db};
use deino_domain::{FakeClock, MeetingId, SystemClock};
use deino_llm::{CompletionRequest, MockProvider, Route, TextSink, mock_gateway};
use deino_testkit::fixture::{FixtureMeeting, Fixtures, MeetingKind, seed_fixtures};

use super::*;
use crate::enhance::MaterialSources;
use crate::recipes::{Recipe, RecipeService, SqliteRecipeRepo};
use crate::redaction::TextRedactor;
use crate::segments::{Page, SegmentReader, SqliteSegmentRepo};

/// Collects the streamed pieces.
#[derive(Default)]
struct Collect(Mutex<String>);

impl TextSink for Collect {
    fn text(&self, piece: &str) {
        self.0.lock().unwrap().push_str(piece);
    }
}

/// Hides one word, like NOT-09's redactor hides a secret.
struct HideWord(&'static str);

#[async_trait]
impl TextRedactor for HideWord {
    async fn redact(&self, texts: Vec<String>) -> Vec<String> {
        texts
            .into_iter()
            .map(|t| t.replace(self.0, "[redacted]"))
            .collect()
    }
}

struct World {
    database: Database,
    service: AskService,
    recipes: Arc<RecipeService>,
    mock: Arc<MockProvider>,
    meeting: FixtureMeeting,
}

fn discovery() -> FixtureMeeting {
    let fixtures = Fixtures::load().unwrap();
    fixtures
        .meeting(MeetingKind::SalesDiscovery)
        .unwrap()
        .clone()
}

/// The `s<n>` of the transcript line in the request's last message that
/// contains `needle`.
fn reference_of(request: &CompletionRequest, needle: &str) -> String {
    let last = request
        .messages
        .last()
        .map(deino_llm::Message::text)
        .unwrap_or_default();
    let line = last
        .lines()
        .find(|line| line.contains(needle))
        .unwrap_or_default();
    line.trim_start_matches('[')
        .split(' ')
        .next()
        .unwrap_or_default()
        .to_owned()
}

async fn world(mock: MockProvider, redactor: Arc<dyn TextRedactor>) -> World {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let mock = Arc::new(mock);
    let gateway = Arc::new(mock_gateway(mock.clone()).gateway);
    let clock = Arc::new(FakeClock::new(chrono::DateTime::UNIX_EPOCH));
    let recipes = Arc::new(RecipeService::new(
        Arc::new(SqliteRecipeRepo::new(database.clone())),
        Arc::new(SystemClock),
    ));
    let service = AskService::new(AskDeps {
        sources: MaterialSources::sqlite(&database).with_redactor(redactor),
        store: Arc::new(SqliteAskRepo::new(database.clone())),
        recipes: recipes.clone(),
        model: Arc::new(GatewayAskModel::new(gateway)),
        clock,
    });
    World {
        database,
        service,
        recipes,
        mock,
        meeting: discovery(),
    }
}

/// Answers with the first fact's evidence line cited, and an invented line.
fn citing_mock() -> MockProvider {
    let evidence = discovery().ground_truth.facts[0].evidence.clone();
    MockProvider::new().answer_with(Route::Chat, move |request| {
        let reference = reference_of(request, &evidence);
        format!("About 220 technicians [{reference}] in three regions [s9999].")
    })
}

fn question(text: &str) -> AskRequest {
    AskRequest {
        question: text.into(),
        recipe: None,
    }
}

async fn recipe(world: &World, slug: &str) -> Recipe {
    let all = world.recipes.list().await.unwrap();
    all.into_iter().find(|recipe| recipe.slug == slug).unwrap()
}

#[tokio::test]
async fn answers_stream_and_cite_only_real_segments_of_the_meeting() {
    let world = world(citing_mock(), Arc::new(crate::redaction::NoRedaction)).await;
    let sink = Collect::default();

    let answer = world
        .service
        .ask(
            &world.meeting.id,
            question("How big is their field team?"),
            &sink,
        )
        .await
        .unwrap();

    assert!(sink.0.lock().unwrap().contains("[s9999]"));
    let cited: Vec<_> = answer
        .parts
        .iter()
        .flat_map(|p| p.citations.clone())
        .collect();
    assert_eq!(cited.len(), 1);
    let segments = SqliteSegmentRepo::new(world.database.clone());
    let all = segments.list(&world.meeting.id, Page::ALL).await.unwrap();
    let segment = all.iter().find(|s| s.id == cited[0].segment_id).unwrap();
    assert!(
        segment
            .text
            .contains(&world.meeting.ground_truth.facts[0].evidence)
    );
    assert_eq!(
        answer.plain_text(),
        "About 220 technicians in three regions."
    );
    let thread = world.service.thread(&world.meeting.id).await.unwrap();
    assert_eq!(
        thread.iter().map(|m| m.role).collect::<Vec<_>>(),
        [AskRole::User, AskRole::Assistant]
    );
}

#[tokio::test]
async fn secrets_in_the_transcript_never_reach_the_model() {
    let world = world(citing_mock(), Arc::new(HideWord("technicians"))).await;

    world
        .service
        .ask(
            &world.meeting.id,
            question("Team size?"),
            &Collect::default(),
        )
        .await
        .unwrap();

    let request = world.mock.requests().pop().unwrap();
    let sent = request.messages.last().unwrap().text();
    assert!(!sent.contains("technicians") && sent.contains("[redacted]"));
}

#[tokio::test]
async fn recipes_chain_inside_a_thread_and_label_the_question() {
    let mock = MockProvider::new()
        .reply(Route::Chat, "Decision one.")
        .reply(Route::Chat, "Dear team, decision one.");
    let world = world(mock, Arc::new(crate::redaction::NoRedaction)).await;
    let decisions = recipe(&world, "extract-decisions").await;
    let email = recipe(&world, "write-follow-up-email").await;
    let run = |id| AskRequest {
        question: String::new(),
        recipe: Some(id),
    };

    world
        .service
        .ask(&world.meeting.id, run(decisions.id), &Collect::default())
        .await
        .unwrap();
    world
        .service
        .ask(&world.meeting.id, run(email.id), &Collect::default())
        .await
        .unwrap();

    let second = world.mock.requests().pop().unwrap();
    let texts: Vec<String> = second
        .messages
        .iter()
        .map(deino_llm::Message::text)
        .collect();
    assert_eq!(
        texts[..2],
        ["(Ran the recipe \"Extract decisions\".)", "Decision one."]
    );
    assert!(texts[2].contains(email.prompt.as_str()));
    let thread = world.service.thread(&world.meeting.id).await.unwrap();
    assert_eq!(
        thread[2].recipe.as_ref().map(|r| r.slug.as_str()),
        Some("write-follow-up-email")
    );
}

#[tokio::test]
async fn preview_saves_nothing() {
    let world = world(
        MockProvider::new().reply(Route::Chat, "Brief."),
        Arc::new(crate::redaction::NoRedaction),
    )
    .await;
    let brief = recipe(&world, "write-a-brief").await;

    let preview = world
        .service
        .preview(&world.meeting.id, &brief.id, &Collect::default())
        .await
        .unwrap();

    assert_eq!(preview.plain_text(), "Brief.");
    assert!(
        world
            .service
            .thread(&world.meeting.id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn empty_questions_multi_meeting_recipes_and_unknown_meetings_are_refused() {
    let world = world(MockProvider::new(), Arc::new(crate::redaction::NoRedaction)).await;
    let recap = recipe(&world, "weekly-recap").await;
    let unknown = MeetingId::new("meeting-none");
    let sink = Collect::default();
    assert!(matches!(
        world
            .service
            .ask(&world.meeting.id, question(" "), &sink)
            .await,
        Err(AskError::EmptyQuestion)
    ));
    assert!(matches!(
        world
            .service
            .preview(&world.meeting.id, &recap.id, &sink)
            .await,
        Err(AskError::Recipe(_))
    ));
    assert!(matches!(
        world.service.ask(&unknown, question("q"), &sink).await,
        Err(AskError::MeetingNotFound(_))
    ));
}

#[tokio::test]
async fn a_thread_keeps_its_newest_messages_and_clears() {
    let world = world(
        MockProvider::new().answer_with(Route::Chat, |_| "ok".into()),
        Arc::new(crate::redaction::NoRedaction),
    )
    .await;

    for n in 0..22 {
        world
            .service
            .ask(
                &world.meeting.id,
                question(&format!("q{n}")),
                &Collect::default(),
            )
            .await
            .unwrap();
    }

    let thread = world.service.thread(&world.meeting.id).await.unwrap();
    assert_eq!(thread.len(), usize::try_from(MAX_THREAD_MESSAGES).unwrap());
    assert_eq!(thread[0].plain_text(), "q2");
    world.service.clear(&world.meeting.id).await.unwrap();
    assert!(
        world
            .service
            .thread(&world.meeting.id)
            .await
            .unwrap()
            .is_empty()
    );
}
