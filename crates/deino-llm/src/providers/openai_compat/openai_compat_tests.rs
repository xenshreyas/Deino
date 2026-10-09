//! Wire tests: recorded OpenAI-compatible streams through the real decoder,
//! the request JSON each flavor sends, and the 400/422 compatibility retry.

use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::{
    FakeHttp, JsonOutput, Message, Part, ProviderId, Role, Route, StaticCredentials, StopReason,
    ToolCall, ToolResult, Usage,
};

fn provider(
    flavor: Flavor,
    http: FakeHttp,
    credentials: StaticCredentials,
) -> (OpenAiCompatProvider, Arc<FakeHttp>) {
    let http = Arc::new(http);
    let credentials = Arc::new(credentials);
    let provider =
        OpenAiCompatProvider::new(flavor, http.clone(), credentials.clone(), credentials);
    (provider, http)
}

fn openai(http: FakeHttp) -> (OpenAiCompatProvider, Arc<FakeHttp>) {
    provider(
        Flavor::OpenAi,
        http,
        StaticCredentials::new().key(ProviderId::OpenAi, "sk-test"),
    )
}

fn request(output: OutputFormat) -> CompletionRequest {
    CompletionRequest {
        route: Route::Title,
        model: "gpt-5-mini".to_owned(),
        system: Some("Title the meeting.".to_owned()),
        messages: vec![Message::user("Transcript")],
        tools: Vec::new(),
        output,
        max_tokens: 64,
        temperature: None,
    }
}

fn title_schema() -> OutputFormat {
    OutputFormat::Json(JsonOutput {
        name: "meeting_title".into(),
        schema: json!({ "type": "object", "properties": { "title": { "type": "string" } } }),
    })
}

#[tokio::test]
async fn a_text_stream_yields_content_usage_and_stop_without_reasoning() {
    let (provider, http) =
        openai(FakeHttp::new().reply(200, include_str!("../../../tests/fixtures/openai_text.sse")));

    let completion = provider
        .complete(request(OutputFormat::Text))
        .await
        .unwrap();

    assert_eq!(completion.text, "Pilot approved.");
    assert_eq!(
        completion.usage,
        Usage {
            input_tokens: 40,
            output_tokens: 3
        }
    );
    assert_eq!(completion.stop, StopReason::EndTurn);
    let sent = &http.requests()[0];
    assert_eq!(sent.url, "https://api.openai.com/v1/chat/completions");
    assert_eq!(sent.header_value("authorization"), Some("Bearer sk-test"));
}

#[tokio::test]
async fn the_request_body_streams_with_a_usage_chunk_and_a_system_message() {
    let (provider, http) = openai(FakeHttp::new());

    assert!(
        provider.stream(request(OutputFormat::Text)).await.is_err(),
        "no reply is scripted"
    );

    assert_eq!(
        http.json_body(0),
        json!({
            "model": "gpt-5-mini",
            "messages": [
                { "role": "system", "content": "Title the meeting." },
                { "role": "user", "content": "Transcript" },
            ],
            "stream": true,
            "stream_options": { "include_usage": true },
        })
    );
}

#[tokio::test]
async fn tool_call_fragments_are_assembled_by_index_even_without_done() {
    let (provider, _) = openai(FakeHttp::new().reply(
        200,
        include_str!("../../../tests/fixtures/openai_tool_calls.sse"),
    ));

    let completion = provider
        .complete(request(OutputFormat::Text))
        .await
        .unwrap();

    assert_eq!(
        completion.tool_calls,
        [
            ToolCall {
                id: "call_a".into(),
                name: "search_meetings".into(),
                input: json!({ "query": "pricing" })
            },
            ToolCall {
                id: "call_b".into(),
                name: "list_people".into(),
                input: json!({})
            },
        ]
    );
    assert_eq!(completion.stop, StopReason::ToolUse);
    assert_eq!(completion.usage.input_tokens, 88);
}

#[tokio::test]
async fn tool_turns_become_assistant_tool_calls_and_tool_messages() {
    let (provider, http) = openai(FakeHttp::new());
    let mut turn = request(OutputFormat::Text);
    turn.messages.push(Message {
        role: Role::Assistant,
        parts: vec![Part::ToolUse(ToolCall {
            id: "call_a".into(),
            name: "search".into(),
            input: json!({ "q": 1 }),
        })],
    });
    turn.messages.push(Message {
        role: Role::User,
        parts: vec![Part::ToolResult(ToolResult {
            call_id: "call_a".into(),
            content: "none".into(),
            is_error: false,
        })],
    });

    assert!(provider.stream(turn).await.is_err(), "no reply is scripted");

    let messages = &http.json_body(0)["messages"];
    assert_eq!(
        messages[2]["tool_calls"][0]["function"],
        json!({ "name": "search", "arguments": "{\"q\":1}" })
    );
    assert_eq!(
        messages[3],
        json!({ "role": "tool", "content": "none", "tool_call_id": "call_a" })
    );
}

#[tokio::test]
async fn json_answers_use_response_format_json_schema() {
    let (provider, http) = openai(FakeHttp::new());

    assert!(
        provider.stream(request(title_schema())).await.is_err(),
        "no reply is scripted"
    );

    let body = http.json_body(0);
    assert_eq!(body["response_format"]["type"], "json_schema");
    assert_eq!(
        body["response_format"]["json_schema"]["name"],
        "meeting_title"
    );
    assert_eq!(body["messages"][0]["content"], "Title the meeting.");
}

#[tokio::test]
async fn a_422_retries_once_without_optional_fields_and_moves_the_schema_into_the_prompt() {
    let http = FakeHttp::new()
        .reply(
            422,
            r#"{"error":{"message":"response_format is not supported"}}"#,
        )
        .reply(200, include_str!("../../../tests/fixtures/openai_text.sse"));
    let (provider, http) = openai(http);

    let completion = provider.complete(request(title_schema())).await.unwrap();

    assert_eq!(completion.text, "Pilot approved.");
    let retry = http.json_body(1);
    assert!(retry.get("response_format").is_none());
    assert!(retry.get("stream_options").is_none());
    let system = retry["messages"][0]["content"].as_str().unwrap();
    assert!(system.starts_with("Title the meeting.\n\nAnswer with only a JSON value"));
}

#[tokio::test]
async fn ollama_needs_no_key_turns_reasoning_off_and_uses_the_configured_endpoint() {
    let credentials = StaticCredentials::new().endpoint(ProviderId::Ollama, "http://gpu-box:11434");
    let (provider, http) = provider(Flavor::Ollama, FakeHttp::new(), credentials);

    assert!(
        provider.stream(request(OutputFormat::Text)).await.is_err(),
        "no reply is scripted"
    );

    let sent = &http.requests()[0];
    assert_eq!(sent.url, "http://gpu-box:11434/v1/chat/completions");
    assert_eq!(sent.header_value("authorization"), None);
    assert_eq!(http.json_body(0)["reasoning_effort"], "none");
}

#[tokio::test]
async fn a_custom_endpoint_without_a_url_is_unavailable() {
    let (provider, http) = provider(Flavor::Custom, FakeHttp::new(), StaticCredentials::new());

    let error = provider
        .stream(request(OutputFormat::Text))
        .await
        .err()
        .unwrap();

    assert!(matches!(
        error,
        LlmError::Unavailable {
            provider: ProviderId::CustomOpenAi,
            ..
        }
    ));
    assert!(http.requests().is_empty());
}

#[tokio::test]
async fn models_are_filtered_per_flavor_and_fall_back_without_a_key() {
    let list = r#"{"data":[{"id":"gpt-5"},{"id":"text-embedding-3-large"},{"id":"whisper-1"}]}"#;
    let (keyed, _) = openai(FakeHttp::new().reply(200, list));
    let (keyless, http) = provider(Flavor::Groq, FakeHttp::new(), StaticCredentials::new());

    let live: Vec<String> = keyed
        .models(&crate::DraftConnection::default())
        .await
        .unwrap()
        .into_iter()
        .map(|model| model.id)
        .collect();
    let fallback: Vec<String> = keyless
        .models(&crate::DraftConnection::default())
        .await
        .unwrap()
        .into_iter()
        .map(|model| model.id)
        .collect();

    assert_eq!(live, ["gpt-5"]);
    assert_eq!(fallback, ["llama-3.3-70b-versatile"]);
    assert!(http.requests().is_empty());
}

#[tokio::test]
async fn openrouter_lists_models_without_a_key() {
    let list = r#"{"data":[{"id":"anthropic/claude-sonnet-5.5","name":"Claude Sonnet 5.5","context_length":1000,"top_provider":{"context_length":200000}}]}"#;
    let (provider, http) = provider(
        Flavor::OpenRouter,
        FakeHttp::new().reply(200, list),
        StaticCredentials::new(),
    );

    let models = provider
        .models(&crate::DraftConnection::default())
        .await
        .unwrap();

    assert_eq!(models[0].context_length, Some(200_000));
    assert_eq!(
        http.requests()[0].url,
        "https://openrouter.ai/api/v1/models"
    );
}
