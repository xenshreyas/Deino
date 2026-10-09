//! The built-in adapter over a scripted sidecar.

use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::{JsonOutput, Route, ToolDefinition};

fn request(messages: Vec<Message>, output: OutputFormat) -> CompletionRequest {
    CompletionRequest {
        route: Route::NotesEnhance,
        model: "qwen3.5:2b".to_owned(),
        system: Some("Summarise.".to_owned()),
        messages,
        tools: Vec::new(),
        output,
        max_tokens: 4096,
        temperature: None,
    }
}

#[tokio::test]
async fn helper_tokens_stream_as_text_and_done_becomes_usage_and_stop() {
    let local = Arc::new(FakeLocalModels::new().reply("Pilot approved."));
    let provider = BuiltInProvider::new(local.clone());

    let completion = provider
        .complete(request(
            vec![Message::user("Transcript")],
            OutputFormat::Text,
        ))
        .await
        .unwrap();

    assert_eq!(completion.text, "Pilot approved.");
    assert_eq!(
        completion.usage,
        Usage {
            input_tokens: 10,
            output_tokens: 2
        }
    );
    assert_eq!(completion.stop, StopReason::EndTurn);
    let (model, generation) = &local.generations()[0];
    assert_eq!(model, "qwen3.5:2b");
    assert_eq!(
        generation,
        &LocalGeneration {
            system: "Summarise.".into(),
            prompt: "Transcript".into(),
            max_tokens: 4096,
            json_schema: None,
        }
    );
}

#[tokio::test]
async fn json_answers_send_the_schema_for_a_grammar() {
    let local = Arc::new(FakeLocalModels::new().reply("{}"));
    let schema = json!({ "type": "object" });
    let output = OutputFormat::Json(JsonOutput {
        name: "answer".into(),
        schema: schema.clone(),
    });

    let _ = BuiltInProvider::new(local.clone())
        .complete(request(vec![Message::user("x")], output))
        .await
        .unwrap();

    assert_eq!(local.generations()[0].1.json_schema, Some(schema));
}

#[tokio::test]
async fn a_conversation_is_flattened_into_turns() {
    let local = Arc::new(FakeLocalModels::new().reply("ok"));
    let messages = vec![
        Message::user("Hi"),
        Message::assistant("Hello"),
        Message::user("Summarise"),
    ];

    BuiltInProvider::new(local.clone())
        .complete(request(messages, OutputFormat::Text))
        .await
        .unwrap();

    assert_eq!(
        local.generations()[0].1.prompt,
        "User: Hi\n\nAssistant: Hello\n\nUser: Summarise"
    );
}

#[tokio::test]
async fn tools_are_unsupported() {
    let mut with_tools = request(vec![Message::user("x")], OutputFormat::Text);
    with_tools.tools.push(ToolDefinition {
        name: "search".into(),
        description: String::new(),
        input_schema: json!({}),
    });

    let error = BuiltInProvider::new(Arc::new(FakeLocalModels::new()))
        .stream(with_tools)
        .await
        .err()
        .unwrap();

    assert!(matches!(
        error,
        LlmError::Unsupported {
            feature: "tool calling",
            ..
        }
    ));
}

#[tokio::test]
async fn a_helper_error_line_ends_the_stream_with_an_error() {
    let local = FakeLocalModels::new().events(vec![
        HelperEvent::Token { text: "Par".into() },
        HelperEvent::Error {
            message: "Generation failed: out of memory".into(),
        },
    ]);

    let error = BuiltInProvider::new(Arc::new(local))
        .complete(request(vec![Message::user("x")], OutputFormat::Text))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("out of memory"));
}

#[test]
fn helper_lines_deserialize_from_the_sidecar_protocol() {
    let done: HelperEvent = serde_json::from_str(
        r#"{"type":"done","prompt_tokens":17,"output_tokens":5,"stop":"stop_token"}"#,
    )
    .unwrap();

    assert_eq!(
        done,
        HelperEvent::Done {
            prompt_tokens: 17,
            output_tokens: 5,
            stop: HelperStop::StopToken
        }
    );
}
