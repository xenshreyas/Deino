//! Smoke tests with a real built-in model: the gateway, the sidecar adapter
//! and the real llama-helper end to end. Ignored by default (they need a
//! downloaded GGUF and take seconds); run them by hand:
//!
//! ```sh
//! DEINO_SMOKE_APP_DATA=<dir holding models/summary/*.gguf> \
//! DEINO_LLAMA_HELPER=target/release/llama-helper \
//!   cargo test -p deino --lib llm::smoke_tests -- --ignored --nocapture
//! ```

use std::sync::Arc;

use deino_core::enhance::EnhancedDoc;
use deino_domain::SystemClock;
use deino_llm::prompts::{EnhanceSystem, EnhanceUser, PromptText};
use deino_llm::{
    BuiltInProvider, CallOutcome, Gateway, GatewayParts, LocalModels, MemoryRouteStore,
    MemoryUsage, Prompt, ProviderId, Providers, Route, Router, StaticCredentials, Target,
};
use deino_testkit::fixture::Fixtures;
use futures_util::StreamExt;
use schemars::JsonSchema;
use serde::Deserialize;

use super::local::SidecarLocalModels;

struct Smoke {
    gateway: Arc<Gateway>,
    usage: Arc<MemoryUsage>,
    target: Target,
}

async fn smoke() -> Smoke {
    let dir = std::env::var("DEINO_SMOKE_APP_DATA").expect("DEINO_SMOKE_APP_DATA is set");
    let local: Arc<dyn LocalModels> = Arc::new(SidecarLocalModels::new(dir.into()));
    let model = local
        .models()
        .await
        .into_iter()
        .find(|model| model.ready)
        .expect("a downloaded built-in model")
        .id;
    let usage = Arc::new(MemoryUsage::default());
    let gateway = Gateway::new(GatewayParts {
        providers: Providers::new().with(
            ProviderId::BuiltIn,
            Arc::new(BuiltInProvider::new(local.clone())),
        ),
        router: Router::new(
            Arc::new(MemoryRouteStore::default()),
            Arc::new(StaticCredentials::new()),
            local.clone(),
        ),
        local,
        usage: usage.clone(),
        clock: Arc::new(SystemClock),
    });
    let target = Target {
        provider: ProviderId::BuiltIn,
        model,
    };
    Smoke {
        gateway: Arc::new(gateway),
        usage,
        target,
    }
}

fn standup_transcript() -> String {
    let fixtures = Fixtures::load().expect("the fixtures load");
    let standup = fixtures
        .meetings()
        .iter()
        .find(|meeting| meeting.id.as_str().ends_with("standup"))
        .expect("the standup fixture");
    (1..)
        .zip(&standup.segments)
        .map(|(n, segment)| format!("[s{n}] {}: {}", segment.speaker, segment.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One test, so one Tokio runtime: the sidecar's pipes belong to the runtime
/// that spawned it, as they do in the app.
#[tokio::test]
#[ignore = "needs a downloaded built-in model and the llama-helper binary"]
async fn the_built_in_model_streams_structures_and_writes_notes_through_the_gateway() {
    let smoke = smoke().await;
    the_route_default_streams_tokens_from_the_real_sidecar(&smoke).await;
    structured_output_uses_the_grammar(&smoke).await;
    enhanced_notes_are_written_through_the_gateway(&smoke).await;
}

async fn the_route_default_streams_tokens_from_the_real_sidecar(smoke: &Smoke) {
    let prompt = Prompt::new("Name three primary colours, comma separated.").max_tokens(24);

    let mut stream = smoke.gateway.stream(Route::Chat, prompt).await.unwrap();
    let mut pieces = 0;
    while let Some(event) = stream.next().await {
        if matches!(event.unwrap(), deino_llm::CompletionEvent::Text(_)) {
            pieces += 1;
        }
    }

    assert!(pieces > 1, "the reply arrived as several pieces");
    assert_eq!(smoke.usage.calls()[0].outcome, CallOutcome::Ok);
    assert!(smoke.usage.calls()[0].usage.output_tokens > 0);
}

/// A meeting title, as structured output.
#[derive(Debug, Deserialize, JsonSchema)]
struct MeetingTitle {
    /// The title.
    title: String,
}

async fn structured_output_uses_the_grammar(smoke: &Smoke) {
    let prompt = Prompt::new(format!(
        "Give this meeting a short title.\n\n{}",
        standup_transcript()
    ))
    .on(smoke.target.clone())
    .max_tokens(64);

    let title: MeetingTitle = smoke
        .gateway
        .structured(Route::Title, prompt)
        .await
        .unwrap();

    assert!(!title.title.trim().is_empty());
    println!("title: {}", title.title);
}

async fn enhanced_notes_are_written_through_the_gateway(smoke: &Smoke) {
    let transcript = standup_transcript();
    let user = EnhanceUser {
        meeting: "Title: Daily standup",
        template: "General. Sections, in order:\n1. Notes: Write bullet blocks, one idea each.",
        previous: None,
        vocabulary: None,
        notes: Some("- release cut today"),
        transcript: &transcript,
    };
    let prompt = Prompt::new(user.text())
        .system(EnhanceSystem.text())
        .on(smoke.target.clone())
        .max_tokens(2_048);

    let notes: EnhancedDoc = smoke
        .gateway
        .structured(Route::NotesEnhance, prompt)
        .await
        .unwrap();

    assert!(!notes.sections.is_empty());
    let calls = smoke.usage.calls();
    let last = calls.last().expect("the notes were metered");
    assert_eq!(
        (last.route, last.outcome),
        (Route::NotesEnhance, CallOutcome::Ok)
    );
    println!("{} sections", notes.sections.len());
}
