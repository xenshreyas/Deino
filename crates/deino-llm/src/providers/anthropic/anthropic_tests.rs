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
            "max_tokens": wire::ANTHROPIC_MIN_TOKENS,
            "system": "Be brief.",
            "messages": [{ "role": "user", "content": [{ "type": "text", "text": "Hi" }] }],
            "stream": true,
        })
    );
}

#[tokio::test]
async fn a_40_token_title_request_goes_out_with_the_floor_and_answers_with_text_only() {
    let fixture = include_str!("../../../tests/fixtures/anthropic_text.sse");
    let (provider, http) = provider(FakeHttp::new().reply(200, fixture));
    let mut title = request(OutputFormat::Text);
    title.route = Route::Title;
    title.model = CLAUDE_FAST_MODEL.to_owned();
    title.max_tokens = 40;

    let completion = provider.complete(title).await.unwrap();

    let body = http.json_body(0);
    assert_eq!(body["max_tokens"], wire::ANTHROPIC_MIN_TOKENS);
    assert_eq!(body["model"], "claude-haiku-5-5");
    assert_eq!(completion.text, "Hello, Shreyas.", "thinking is not text");
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

fn json_output(schema: serde_json::Value) -> OutputFormat {
    OutputFormat::Json(JsonOutput {
        name: "meeting_title".into(),
        schema,
    })
}

fn title_output() -> OutputFormat {
    json_output(json!({
        "type": "object",
        "properties": { "title": { "type": "string", "maxLength": 80 } },
        "required": ["title"]
    }))
}

#[tokio::test]
async fn a_json_answer_is_a_structured_output_without_tools_or_temperature() {
    let (provider, http) = provider(FakeHttp::new());
    let mut asked = request(title_output());
    asked.temperature = Some(0.2);
    asked.tools = vec![ToolDefinition {
        name: "search_meetings".into(),
        description: "Search past meetings.".into(),
        input_schema: json!({ "type": "object" }),
    }];

    assert!(
        provider.stream(asked).await.is_err(),
        "no reply is scripted"
    );

    assert_eq!(
        http.json_body(0),
        json!({
            "model": "claude-sonnet-5-5",
            "max_tokens": wire::ANTHROPIC_MIN_TOKENS,
            "system": "Be brief.",
            "messages": [{ "role": "user", "content": [{ "type": "text", "text": "Hi" }] }],
            "output_config": {
                "format": {
                    "type": "json_schema",
                    "schema": {
                        "type": "object",
                        "properties": { "title": { "type": "string" } },
                        "required": ["title"],
                        "additionalProperties": false
                    }
                }
            },
            "stream": true,
        })
    );
}

#[tokio::test]
async fn a_json_answer_keeps_a_cap_above_the_floor() {
    let (provider, http) = provider(FakeHttp::new());
    let mut asked = request(title_output());
    asked.max_tokens = 32_000;

    assert!(
        provider.stream(asked).await.is_err(),
        "no reply is scripted"
    );

    assert_eq!(http.json_body(0)["max_tokens"], 32_000);
}

#[tokio::test]
async fn a_schema_structured_outputs_cannot_take_is_asked_for_in_the_system_prompt() {
    let (provider, http) = provider(FakeHttp::new());
    let recursive = json!({
        "type": "object",
        "properties": { "children": { "type": "array", "items": { "$ref": "#" } } }
    });

    assert!(
        provider
            .stream(request(json_output(recursive.clone())))
            .await
            .is_err(),
        "no reply is scripted"
    );

    let body = http.json_body(0);
    assert!(body.get("output_config").is_none());
    assert!(body.get("tools").is_none());
    assert!(body.get("tool_choice").is_none());
    let system = body["system"].as_str().unwrap();
    assert!(system.starts_with("Be brief.\n\nReply with only one JSON value"));
    assert!(system.ends_with(&recursive.to_string()));
}

#[tokio::test]
async fn a_json_answer_streams_its_text_after_thinking() {
    let fixture = include_str!("../../../tests/fixtures/anthropic_json_answer.sse");
    let (provider, _) = provider(FakeHttp::new().reply(200, fixture));

    let completion = provider.complete(request(title_output())).await.unwrap();

    assert_eq!(completion.text, r#"{"title": "Brightkite pilot scope"}"#);
    assert!(completion.tool_calls.is_empty());
    assert_eq!(completion.stop, StopReason::EndTurn);
    assert_eq!(
        completion.usage,
        Usage {
            input_tokens: 120,
            output_tokens: 48
        }
    );
}

#[tokio::test]
async fn a_json_answer_cut_off_at_max_tokens_is_truncated_after_its_usage() {
    use futures_util::StreamExt;
    let fixture = include_str!("../../../tests/fixtures/anthropic_json_max_tokens.sse");
    let (provider, _) = provider(FakeHttp::new().reply(200, fixture));

    let events: Vec<Result<CompletionEvent, LlmError>> = provider
        .stream(request(title_output()))
        .await
        .unwrap()
        .collect()
        .await;

    let usage = Usage {
        input_tokens: 900,
        output_tokens: 16_000,
    };
    assert!(matches!(&events[..], [
        Ok(CompletionEvent::Text(_)),
        Ok(CompletionEvent::Usage(reported)),
        Err(LlmError::Truncated),
    ] if *reported == usage));
}

#[tokio::test]
async fn a_text_answer_cut_off_at_max_tokens_still_ends_normally() {
    let fixture = include_str!("../../../tests/fixtures/anthropic_json_max_tokens.sse");
    let (provider, _) = provider(FakeHttp::new().reply(200, fixture));

    let completion = provider
        .complete(request(OutputFormat::Text))
        .await
        .unwrap();

    assert_eq!(completion.stop, StopReason::MaxTokens);
}

#[tokio::test]
async fn text_and_tool_requests_never_send_a_temperature() {
    let (provider, http) = provider(FakeHttp::new());
    let mut asked = request(OutputFormat::Text);
    asked.temperature = Some(0.2);
    asked.tools = vec![ToolDefinition {
        name: "search_meetings".into(),
        description: "Search past meetings.".into(),
        input_schema: json!({ "type": "object" }),
    }];

    assert!(
        provider.stream(asked).await.is_err(),
        "no reply is scripted"
    );

    let body = http.json_body(0);
    assert!(body.get("temperature").is_none());
    assert!(body.get("tool_choice").is_none());
    assert!(body.get("output_config").is_none());
    assert_eq!(body["max_tokens"], wire::ANTHROPIC_MIN_TOKENS);
    assert_eq!(
        body["tools"][0]["input_schema"],
        json!({ "type": "object" })
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
