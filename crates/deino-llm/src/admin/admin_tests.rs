//! The Settings → AI service over fakes.

use std::sync::{Arc, Mutex};

use deino_domain::FakeClock;
use deino_testkit::at;

use super::*;
use crate::{
    FakeHttp, MOCK_LOCAL_MODEL, MockProvider, RouteSource, StaticCredentials, mock_gateway,
};

#[derive(Default)]
struct Collected(Mutex<String>);

impl TextSink for Collected {
    fn text(&self, piece: &str) {
        self.0.lock().unwrap().push_str(piece);
    }
}

fn admin(mock: MockProvider, credentials: StaticCredentials) -> LlmAdmin {
    let credentials = Arc::new(credentials);
    let http = Arc::new(FakeHttp::new());
    let ollama = Arc::new(OllamaProvider::new(
        http,
        credentials.clone(),
        credentials.clone(),
    ));
    let mut catalogs: BTreeMap<ProviderId, Arc<dyn ModelCatalog>> = BTreeMap::new();
    catalogs.insert(ProviderId::Ollama, ollama.clone());
    LlmAdmin::new(AdminParts {
        gateway: Arc::new(mock_gateway(Arc::new(mock)).gateway),
        catalogs,
        credentials: credentials.clone(),
        endpoints: credentials,
        ollama,
        clock: Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
    })
}

#[tokio::test]
async fn providers_show_masked_keys_and_endpoints_only() {
    let credentials = StaticCredentials::new()
        .key(ProviderId::Anthropic, "sk-ant-api03-secretvalue-7f3c")
        .endpoint(ProviderId::Ollama, "http://gpu-box:11434");

    let providers = admin(MockProvider::new(), credentials)
        .providers()
        .await
        .unwrap();

    let anthropic = &providers[0];
    assert_eq!(anthropic.masked_key.as_deref(), Some("sk-a…7f3c"));
    assert!(anthropic.requires_key);
    let ollama = providers
        .iter()
        .find(|status| status.id == ProviderId::Ollama)
        .unwrap();
    assert_eq!(ollama.endpoint.as_deref(), Some("http://gpu-box:11434"));
    assert_eq!(ollama.masked_key, None);
    assert_eq!(providers.len(), ProviderId::ALL.len());
}

#[tokio::test]
async fn a_route_test_streams_the_reply_to_the_sink_and_reports_metadata() {
    let mock = MockProvider::new().reply(Route::Chat, "Hello from the model.");
    let admin = admin(mock, StaticCredentials::new());
    let sink = Collected::default();

    let report = admin.test_route(Route::Chat, &sink).await.unwrap();

    assert_eq!(*sink.0.lock().unwrap(), "Hello from the model.");
    assert_eq!(report.target.model, MOCK_LOCAL_MODEL);
    assert_eq!(report.usage.output_tokens, 4);
    assert_eq!(report.dimensions, None);
}

#[tokio::test]
async fn the_embed_route_test_reports_dimensions() {
    let admin = admin(MockProvider::new(), StaticCredentials::new());

    let report = admin
        .test_route(Route::Embed, &Collected::default())
        .await
        .unwrap();

    assert_eq!(report.dimensions, Some(2));
}

#[tokio::test]
async fn routes_can_be_pinned_and_reset() {
    let admin = admin(MockProvider::new(), StaticCredentials::new());
    let target = Target {
        provider: ProviderId::Groq,
        model: "llama-3.3-70b-versatile".into(),
    };

    let pinned = admin
        .set_route(Route::Tagging, Some(target.clone()))
        .await
        .unwrap();
    let table = admin.routes().await.unwrap();

    assert_eq!(pinned.source, RouteSource::Custom);
    assert_eq!(
        table
            .iter()
            .find(|row| row.route == Route::Tagging)
            .unwrap()
            .target,
        target
    );
}

#[tokio::test]
async fn providers_without_a_catalog_cannot_list_models() {
    let admin = admin(MockProvider::new(), StaticCredentials::new());

    let error = admin
        .models(ProviderId::Anthropic, &crate::DraftConnection::default())
        .await
        .unwrap_err();

    assert!(matches!(error, LlmError::Unsupported { .. }));
}
