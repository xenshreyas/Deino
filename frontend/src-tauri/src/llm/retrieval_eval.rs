//! AI-01's acceptance eval: ten paraphrased questions over the fixture
//! meetings, with the real embedding model, llama-helper and the whole
//! search stack. Each question must find its meeting in the top three
//! meetings; eight of ten must pass. Ignored by default (it needs the
//! model and takes seconds); run it by hand:
//!
//! ```sh
//! DEINO_SMOKE_APP_DATA=<dir holding models/embedding/nomic-embed-text-v1.5.Q8_0.gguf> \
//! DEINO_LLAMA_HELPER=target/release/llama-helper \
//!   cargo test -p deino --lib llm::retrieval_eval -- --ignored --nocapture
//! ```

use std::sync::Arc;
use std::time::Duration;

use deino_core::models::{
    Catalog, FakeByteSource, FakeServer, ModelPurpose, ModelService, ModelServiceParts, ModelStore,
    RecordedModelEvents,
};
use deino_core::search::semantic::{
    GatewayEmbedder, IndexerParts, SemanticIndex, SemanticIndexer, SemanticSearch, SharedVectors,
    SqliteEmbeddingStore,
};
use deino_core::search::{SearchFilters, SearchService, SqliteSearchIndex};
use deino_domain::{MeetingId, SystemClock};
use deino_llm::{
    Gateway, GatewayParts, LocalModels, MemoryRouteStore, MemoryUsage, Providers, Router,
    StaticCredentials,
};
use deino_testkit::{seed_fixtures, test_db};

use super::local::SidecarLocalModels;

/// The questions and the fixture meeting each one is about.
const QUESTIONS: [(&str, &str); 10] = [
    (
        "What compensation range is the backend candidate hoping for?",
        "interview",
    ),
    (
        "How much money are we crediting the customer whose technicians were sent out twice?",
        "customer-support",
    ),
    ("When does Maya rejoin the on-call schedule?", "one-on-one"),
    (
        "How dependent is our revenue on the largest few accounts?",
        "board-update",
    ),
    (
        "What limits did we agree on for uploading spreadsheets of customer data?",
        "brainstorm",
    ),
    (
        "What does the prospect's security lead need before signing?",
        "sales-discovery",
    ),
    ("Why is the automated test suite flaky lately?", "standup"),
    (
        "How much engineering time do we really have next quarter compared to everything pitched?",
        "planning",
    ),
    (
        "What should Maya work on to reach the next engineering level?",
        "one-on-one",
    ),
    (
        "In which cloud regions do we keep customer data?",
        "sales-discovery",
    ),
];
/// Questions that must find their meeting in the top three.
const REQUIRED: usize = 8;

fn gateway(app_data: &str, models: Arc<ModelService>) -> Arc<Gateway> {
    let local: Arc<dyn LocalModels> =
        Arc::new(SidecarLocalModels::new(app_data.into()).with_embedding_models(Some(models)));
    Arc::new(Gateway::new(GatewayParts {
        providers: Providers::new(),
        router: Router::new(
            Arc::new(MemoryRouteStore::default()),
            Arc::new(StaticCredentials::new()),
            local.clone(),
        ),
        local,
        usage: Arc::new(MemoryUsage::default()),
        clock: Arc::new(SystemClock),
    }))
}

fn embedding_models(app_data: &str) -> Arc<ModelService> {
    let service = ModelService::new(ModelServiceParts {
        catalog: Catalog::bundled().unwrap(),
        store: ModelStore::new(std::path::Path::new(app_data).join("models/embedding")),
        source: Arc::new(FakeByteSource::new(Vec::new(), FakeServer::Ranged)),
        events: Arc::new(RecordedModelEvents::default()),
        memory_bytes: 0,
    });
    Arc::new(service.serving(ModelPurpose::Embedding))
}

/// Where `expected` ranks among the meetings `found` (1-based), if at all.
fn rank_of(found: &[MeetingId], expected: &str) -> Option<usize> {
    let id = format!("meeting-fixture-{expected}");
    found
        .iter()
        .position(|meeting| meeting.as_str() == id)
        .map(|index| index + 1)
}

/// The fixtures seeded, embedded with the real model, and searchable.
async fn indexed_fixtures(app_data: &str) -> (SearchService, Arc<SemanticIndex>) {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let models = embedding_models(app_data);
    let embedder = Arc::new(GatewayEmbedder::new(
        gateway(app_data, models.clone()),
        models,
    ));
    let store = Arc::new(SqliteEmbeddingStore::new(&database));
    let vectors = Arc::new(SharedVectors::new(store.clone(), "nomic-embed-text-v1.5"));
    let semantic_indexer = SemanticIndexer::new(IndexerParts {
        sources: store.clone(),
        store: store.clone(),
        embedder: embedder.clone(),
        vectors: vectors.clone(),
        clock: Arc::new(SystemClock),
    });
    for meeting in deino_testkit::Fixtures::load().unwrap().meetings() {
        let report = semantic_indexer.index_meeting(&meeting.id).await.unwrap();
        println!("{}: {} chunks", meeting.id, report.chunks);
    }
    let semantic = Arc::new(
        SemanticIndex::new(store, embedder, vectors).with_deadline(Duration::from_mins(1)),
    );
    let keyword = Arc::new(SqliteSearchIndex::new(&database));
    let search = SearchService::new(keyword.clone(), keyword, semantic.clone());
    (search, semantic)
}

#[tokio::test]
#[ignore = "needs the embedding model and llama-helper; run by hand (see the module docs)"]
async fn paraphrased_questions_find_their_fixture_meeting_in_the_top_three() {
    let app_data = std::env::var("DEINO_SMOKE_APP_DATA").expect("DEINO_SMOKE_APP_DATA is set");
    let (search, semantic) = indexed_fixtures(&app_data).await;

    let mut passed = 0;
    for (question, expected) in QUESTIONS {
        let hits = search
            .search_hybrid(question, &SearchFilters::default(), 30)
            .await
            .unwrap();
        let hybrid = distinct_meetings(hits.into_iter().filter_map(|hit| hit.meeting_id));
        let semantic_only = semantic_meetings(&semantic, question).await;
        let rank = rank_of(&hybrid, expected);
        passed += usize::from(rank.is_some_and(|rank| rank <= 3));
        println!(
            "{rank:?} (semantic only {:?})  {expected:<16} {question}",
            rank_of(&semantic_only, expected)
        );
    }
    println!(
        "{passed} of {} questions found their meeting in the top 3",
        QUESTIONS.len()
    );
    assert!(passed >= REQUIRED, "{passed} of {} passed", QUESTIONS.len());
}

/// The meetings of the semantic hits alone, best first, each once.
async fn semantic_meetings(semantic: &SemanticIndex, question: &str) -> Vec<MeetingId> {
    let hits = semantic
        .search(question, None, 30)
        .await
        .unwrap()
        .unwrap_or_default();
    distinct_meetings(hits.into_iter().filter_map(|hit| hit.meeting_id))
}

/// `meetings` in order, each once.
fn distinct_meetings(meetings: impl Iterator<Item = MeetingId>) -> Vec<MeetingId> {
    let mut distinct: Vec<MeetingId> = Vec::new();
    for meeting in meetings {
        if !distinct.contains(&meeting) {
            distinct.push(meeting);
        }
    }
    distinct
}
