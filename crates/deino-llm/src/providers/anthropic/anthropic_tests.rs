//! Wire tests: recorded Anthropic streams through the real decoder, and the
//! request JSON Anthropic receives. No network.

use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::{
    CompletionEvent, FakeHttp, JsonOutput, Message, Part, Route, StaticCredentials, StopReason,
    ToolCall, ToolDefinition, ToolResult, Usage,
};

fn provider(http: FakeHttp) -> (AnthropicProvider, Arc<FakeHttp>) {
    let http = Arc::new(http);
    let credentials = StaticCredentials::new().key(PROVIDER, "sk-ant-test");
    let provider = AnthropicProvider::new(http.clone(), Arc::new(credentials));
    (provider, http)
}

fn request(output: OutputFormat) -> CompletionRequest {
    CompletionRequest {
        route: Route::Chat,
        model: CLAUDE_SMART_MODEL.to_owned(),
        system: Some("Be brief.".to_owned()),
        messages: vec![Message::user("Hi")],
        tools: Vec::new(),
        output,
        max_tokens: 256,
        temperature: None,
    }
}

#[tokio::test]
async fn a_text_stream_yields_text_usage_and_stop_and_drops_thinking() {
    let fixture = include_str!("../../../tests/fixtures/anthropic_text.sse");
    let (provider, http) = provider(FakeHttp::new().reply(200, fixture));

    let completion = provider
        .complete(request(OutputFormat::Text))
        .await
        .unwrap();

    assert_eq!(completion.text, "Hello, Shreyas.");
    assert!(completion.tool_calls.is_empty());
    assert_eq!(
        completion.usage,
        Usage {
            input_tokens: 25,
            output_tokens: 15
        }
    );
    assert_eq!(completion.stop, StopReason::EndTurn);
    let sent = &http.requests()[0];
    assert_eq!(sent.url, MESSAGES_URL);
    assert_eq!(sent.header_value("x-api-key"), Some("sk-ant-test"));
    assert_eq!(sent.header_value("anthropic-version"), Some(API_VERSION));
}

#[tokio::test]
async fn the_request_body_is_a_streamed_messages_request() {
    let (provider, http) = provider(FakeHttp::new());

    assert!(
        provider.stream(request(OutputFormat::Text)).await.is_err(),
        "no reply is scripted"
    );

    assert_eq!(
        http.json_body(0),
        json!({
            "model": "claude-sonnet-5-5",
            "max_tokens": 256,
            "system": "Be brief.",
            "messages": [{ "role": "user", "content": [{ "type": "text", "text": "Hi" }] }],
            "stream": true,
        })
    );
}

#[tokio::test]
async fn tool_input_fragments_become_one_tool_use() {
    let fixture = include_str!("../../../tests/fixtures/anthropic_tool_use.sse");
    let (provider, _) = provider(FakeHttp::new().reply(200, fixture));

    let completion = provider
        .complete(request(OutputFormat::Text))
        .await
        .unwrap();

    assert_eq!(completion.text, "Let me search.");
    assert_eq!(
        completion.tool_calls,
        [ToolCall {
            id: "toolu_01".into(),
            name: "search_meetings".into(),
            input: json!({ "query": "pilot pricing" }),
        }]
    );
    assert_eq!(completion.stop, StopReason::ToolUse);
}

#[tokio::test]
async fn tools_and_tool_results_use_anthropic_content_blocks() {
    let (provider, http) = provider(FakeHttp::new());
    let call = ToolCall {
        id: "toolu_01".into(),
        name: "search_meetings".into(),
        input: json!({ "query": "pilot" }),
    };
    let mut tool_request = request(OutputFormat::Text);
    tool_request.tools = vec![ToolDefinition {
        name: "search_meetings".into(),
        description: "Search past meetings.".into(),
        input_schema: json!({ "type": "object" }),
    }];
    tool_request.messages.push(Message {
        role: crate::Role::Assistant,
        parts: vec![Part::ToolUse(call)],
    });
    tool_request.messages.push(Message {
        role: crate::Role::User,
        parts: vec![Part::ToolResult(ToolResult {
            call_id: "toolu_01".into(),
            content: "2 meetings".into(),
            is_error: false,
        })],
    });

    assert!(
        provider.stream(tool_request).await.is_err(),
        "no reply is scripted"
    );

    let body = http.json_body(0);
    assert_eq!(body["tools"][0]["name"], "search_meetings");
    assert_eq!(
        body["tools"][0]["input_schema"],
        json!({ "type": "object" })
    );
    assert_eq!(body["messages"][1]["content"][0]["type"], "tool_use");
    assert_eq!(
        body["messages"][2]["content"][0],
        json!({ "type": "tool_result", "tool_use_id": "toolu_01", "content": "2 meetings", "is_error": false })
    );
    assert!(body.get("tool_choice").is_none());
}

#[tokio::test]
async fn a_json_answer_forces_the_answer_tool_and_streams_its_input_as_text() {
    let fixture = include_str!("../../../tests/fixtures/anthropic_json_answer.sse");
    let (provider, http) = provider(FakeHttp::new().reply(200, fixture));
    let output = OutputFormat::Json(JsonOutput {
        name: "meeting_title".into(),
        schema: json!({ "type": "object", "properties": { "title": { "type": "string" } } }),
    });

    let completion = provider.complete(request(output)).await.unwrap();

    assert_eq!(completion.text, r#"{"title": "Brightkite pilot scope"}"#);
    assert!(completion.tool_calls.is_empty());
    assert_eq!(completion.stop, StopReason::EndTurn);
    let body = http.json_body(0);
    assert_eq!(
        body["tool_choice"],
        json!({ "type": "tool", "name": "meeting_title" })
    );
    assert_eq!(
        body["tools"][0]["input_schema"]["properties"]["title"]["type"],
        "string"
    );
}

#[tokio::test]
async fn an_error_event_mid_stream_ends_the_stream_as_overloaded() {
    let fixture = include_str!("../../../tests/fixtures/anthropic_overloaded.sse");
    let (provider, _) = provider(FakeHttp::new().reply(200, fixture));

    let error = provider
        .complete(request(OutputFormat::Text))
        .await
        .unwrap_err();

    assert!(matches!(error, LlmError::Overloaded { .. }));
}

#[tokio::test]
async fn http_errors_are_classified_before_streaming() {
    let (provider, _) = provider(FakeHttp::new().reply_with_headers(
        429,
        &[("retry-after", "30")],
        r#"{"type":"error","error":{"type":"rate_limit_error","message":"Slow down"}}"#,
    ));

    let error = provider
        .stream(request(OutputFormat::Text))
        .await
        .err()
        .unwrap();

    assert_eq!(
        error.retry_after(),
        Some(std::time::Duration::from_secs(30))
    );
}

#[tokio::test]
async fn without_a_key_nothing_is_sent() {
    let http = Arc::new(FakeHttp::new());
    let provider = AnthropicProvider::new(http.clone(), Arc::new(StaticCredentials::new()));

    let error = provider
        .stream(request(OutputFormat::Text))
        .await
        .err()
        .unwrap();

    assert!(matches!(error, LlmError::NoKey(ProviderId::Anthropic)));
    assert!(http.requests().is_empty());
}

#[tokio::test]
async fn models_list_the_account_s_claude_models_and_defaults_without_a_key() {
    let list = r#"{"data":[{"id":"claude-sonnet-5-5","display_name":"Claude Sonnet 5.5"},{"id":"other"}]}"#;
    let (provider, _) = provider(FakeHttp::new().reply(200, list));
    let keyless = AnthropicProvider::new(
        Arc::new(FakeHttp::new()),
        Arc::new(StaticCredentials::new()),
    );

    let live = provider
        .models(&crate::DraftConnection::default())
        .await
        .unwrap();
    let defaults = keyless
        .models(&crate::DraftConnection::default())
        .await
        .unwrap();

    assert_eq!(live.len(), 1);
    assert_eq!(live[0].display_name.as_deref(), Some("Claude Sonnet 5.5"));
    assert_eq!(defaults.len(), 2);
}

#[tokio::test]
async fn completion_events_arrive_in_order() {
    use futures_util::StreamExt;
    let fixture = include_str!("../../../tests/fixtures/anthropic_text.sse");
    let (provider, _) = provider(FakeHttp::new().reply(200, fixture));

    let events: Vec<CompletionEvent> = provider
        .stream(request(OutputFormat::Text))
        .await
        .unwrap()
        .map(Result::unwrap)
        .collect()
        .await;

    assert_eq!(
        events,
        [
            CompletionEvent::Text("Hello".into()),
            CompletionEvent::Text(", Shreyas.".into()),
            CompletionEvent::Usage(Usage {
                input_tokens: 25,
                output_tokens: 15
            }),
            CompletionEvent::Stop(StopReason::EndTurn),
        ]
    );
}

#[tokio::test]
async fn a_picture_goes_as_a_base64_image_block() {
    let (provider, http) = provider(FakeHttp::new());
    let mut described = request(OutputFormat::Text);
    described.route = Route::VisionDescribe;
    described.messages = vec![Message {
        role: crate::Role::User,
        parts: vec![
            Part::Image(crate::ImagePart {
                media_type: "image/png".to_owned(),
                base64: "iVBORw==".to_owned(),
            }),
            Part::Text("Describe this shared screen.".to_owned()),
        ],
    }];

    assert!(
        provider.stream(described).await.is_err(),
        "no reply is scripted"
    );

    assert_eq!(
        http.json_body(0)["messages"][0]["content"],
        json!([
            {
                "type": "image",
                "source": { "type": "base64", "media_type": "image/png", "data": "iVBORw==" }
            },
            { "type": "text", "text": "Describe this shared screen." }
        ])
    );
}
