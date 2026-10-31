//! Gateway tests over `MockProvider`: routing, pinned targets, metering.

use std::sync::Arc;

use chrono::TimeDelta;
use futures_util::StreamExt;

use crate::{
    CallOutcome, CompletionEvent, LlmError, LlmErrorKind, MOCK_LOCAL_MODEL, MockProvider, Prompt,
    ProviderId, Route, StopReason, Target, Usage, mock_gateway,
};

#[tokio::test]
async fn complete_resolves_the_route_and_sends_the_prompt() {
    let mock = Arc::new(MockProvider::new().reply(Route::Title, "Brightkite pilot"));
    let harness = mock_gateway(mock.clone());

    let completion = harness
        .gateway
        .complete(Route::Title, Prompt::new("transcript").system("Title it."))
        .await
        .unwrap();

    assert_eq!(completion.text, "Brightkite pilot");
    let request = &mock.requests()[0];
    assert_eq!(request.route, Route::Title);
    assert_eq!(request.model, MOCK_LOCAL_MODEL);
    assert_eq!(request.system.as_deref(), Some("Title it."));
    assert_eq!(request.max_tokens, crate::DEFAULT_MAX_TOKENS);
}

#[tokio::test]
async fn a_pinned_target_bypasses_the_route() {
    let mock = Arc::new(MockProvider::new().reply(Route::NotesEnhance, "notes"));
    let harness = mock_gateway(mock.clone());
    let gpt = Target {
        provider: ProviderId::OpenAi,
        model: "gpt-5".into(),
        effort: None,
    };

    harness
        .gateway
        .complete(Route::NotesEnhance, Prompt::new("t").on(gpt))
        .await
        .unwrap();

    assert_eq!(mock.requests()[0].model, "gpt-5");
    assert_eq!(harness.usage.calls()[0].provider, ProviderId::OpenAi);
}

#[tokio::test]
async fn a_finished_call_is_metered_with_usage_and_latency_but_no_content() {
    let mock = Arc::new(MockProvider::new().reply(Route::Chat, "one two three"));
    let harness = mock_gateway(mock);
    let mut stream = harness
        .gateway
        .stream(Route::Chat, Prompt::new("hi"))
        .await
        .unwrap();

    harness.clock.advance(TimeDelta::milliseconds(840));
    while stream.next().await.is_some() {}

    let calls = harness.usage.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].route, Route::Chat);
    assert_eq!(calls[0].model, MOCK_LOCAL_MODEL);
    assert_eq!(
        calls[0].usage,
        Usage {
            input_tokens: 100,
            output_tokens: 3
        }
    );
    assert_eq!(calls[0].latency_ms, 840);
    assert_eq!(calls[0].outcome, CallOutcome::Ok);
}

#[tokio::test]
async fn a_dropped_stream_is_metered_as_cancelled() {
    let mock = Arc::new(MockProvider::new().reply(Route::Chat, "long answer"));
    let harness = mock_gateway(mock);

    let mut stream = harness
        .gateway
        .stream(Route::Chat, Prompt::new("hi"))
        .await
        .unwrap();
    let _first = stream.next().await;
    drop(stream);

    assert_eq!(harness.usage.calls()[0].outcome, CallOutcome::Cancelled);
}

#[tokio::test]
async fn a_failed_call_is_metered_with_its_error_kind() {
    let mock =
        Arc::new(MockProvider::new().fail(Route::Brief, LlmError::Auth(ProviderId::Anthropic)));
    let harness = mock_gateway(mock);

    let error = harness
        .gateway
        .complete(Route::Brief, Prompt::new("x"))
        .await
        .unwrap_err();

    assert!(matches!(error, LlmError::Auth(_)));
    assert_eq!(
        harness.usage.calls()[0].outcome,
        CallOutcome::Error(LlmErrorKind::Auth)
    );
}

#[tokio::test]
async fn a_stream_that_ends_without_a_stop_is_truncated() {
    let mock = Arc::new(
        MockProvider::new().events(Route::Chat, vec![CompletionEvent::Text("Hal".into())]),
    );
    let harness = mock_gateway(mock);

    let error = harness
        .gateway
        .complete(Route::Chat, Prompt::new("x"))
        .await
        .unwrap_err();

    assert!(matches!(error, LlmError::Truncated));
    assert_eq!(
        harness.usage.calls()[0].outcome,
        CallOutcome::Error(LlmErrorKind::Truncated)
    );
}

#[tokio::test]
async fn embed_is_local_and_metered_on_the_embed_route() {
    let harness = mock_gateway(Arc::new(MockProvider::new()));

    let vectors = harness
        .gateway
        .embed(vec!["a".into(), "b".into()])
        .await
        .unwrap();

    assert_eq!(vectors.len(), 2);
    let call = &harness.usage.calls()[0];
    assert_eq!(
        (call.route, call.provider),
        (Route::Embed, ProviderId::BuiltIn)
    );
    assert_eq!(call.model, crate::LOCAL_EMBEDDING_MODEL);
}

#[tokio::test]
async fn stream_yields_events_in_order() {
    let mock = Arc::new(MockProvider::new().reply(Route::Chat, "hi"));
    let harness = mock_gateway(mock);

    let events: Vec<CompletionEvent> = harness
        .gateway
        .stream(Route::Chat, Prompt::new("x"))
        .await
        .unwrap()
        .map(Result::unwrap)
        .collect()
        .await;

    assert_eq!(
        events.last(),
        Some(&CompletionEvent::Stop(StopReason::EndTurn))
    );
}

/// A fixed system context, standing in for the user's profile.
struct AboutMe;

#[async_trait::async_trait]
impl crate::SystemContext for AboutMe {
    async fn system_context(&self) -> Option<String> {
        Some("About the user: CTO".into())
    }
}

#[tokio::test]
async fn the_system_context_leads_every_system_prompt() {
    let mock = Arc::new(
        MockProvider::new()
            .reply(Route::Title, "t")
            .reply(Route::Chat, "c"),
    );
    let gateway = mock_gateway(mock.clone())
        .gateway
        .with_system_context(Arc::new(AboutMe));

    gateway
        .complete(Route::Title, Prompt::new("x").system("Title it."))
        .await
        .unwrap();
    gateway
        .complete(Route::Chat, Prompt::new("hi"))
        .await
        .unwrap();

    let requests = mock.requests();
    assert_eq!(
        requests[0].system.as_deref(),
        Some("About the user: CTO\n\nTitle it.")
    );
    assert_eq!(requests[1].system.as_deref(), Some("About the user: CTO"));
}
