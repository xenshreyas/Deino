//! Structured output over `MockProvider`: schema, parsing and the repair retry.

use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;

use super::*;
use crate::{CallOutcome, MockProvider, Part, mock_gateway};

/// An action item, as AI-05 will extract them.
#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
struct ActionItem {
    /// What to do.
    title: String,
    /// Who owns it.
    owner: Option<Owner>,
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
struct Owner {
    email: String,
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
struct ActionItems {
    items: Vec<ActionItem>,
}

#[test]
fn schemas_are_draft_07_with_every_subschema_inlined() {
    let schema = json_schema_for::<ActionItems>();

    assert!(schema.get("$schema").is_none());
    assert!(schema.get("definitions").is_none());
    let rendered = schema.to_string();
    assert!(!rendered.contains("$ref"), "{rendered}");
    let owner = &schema["properties"]["items"]["items"]["properties"]["owner"];
    assert_eq!(owner["properties"]["email"]["type"], "string", "{owner}");
}

#[test]
fn schema_names_are_snake_case() {
    assert_eq!(schema_name::<ActionItems>(), "action_items");
}

#[test]
fn json_is_found_inside_a_fence_or_prose() {
    let parsed: Owner = parse("Here you go:\n```json\n{\"email\": \"ana@acme.com\"}\n```").unwrap();

    assert_eq!(parsed.email, "ana@acme.com");
}

#[tokio::test]
async fn a_valid_answer_parses_on_the_first_try_and_asks_for_json() {
    let answer = r#"{"items":[{"title":"Send the proposal","owner":{"email":"lena@deino.dev"}}]}"#;
    let mock = Arc::new(MockProvider::new().reply(Route::ExtractActionItems, answer));
    let harness = mock_gateway(mock.clone());

    let items: ActionItems = harness
        .gateway
        .structured(Route::ExtractActionItems, Prompt::new("transcript"))
        .await
        .unwrap();

    assert_eq!(items.items[0].title, "Send the proposal");
    let request = &mock.requests()[0];
    let OutputFormat::Json(json) = &request.output else {
        panic!("structured requests ask for JSON");
    };
    assert_eq!(json.name, "action_items");
    assert_eq!(json.schema, json_schema_for::<ActionItems>());
}

#[tokio::test]
async fn an_invalid_answer_is_repaired_once_with_the_parse_error() {
    let mock = Arc::new(
        MockProvider::new()
            .reply(Route::ExtractActionItems, r#"{"items":[{"name":"x"}]}"#)
            .reply(Route::ExtractActionItems, r#"{"items":[{"title":"x"}]}"#),
    );
    let harness = mock_gateway(mock.clone());

    let items: ActionItems = harness
        .gateway
        .structured(Route::ExtractActionItems, Prompt::new("transcript"))
        .await
        .unwrap();

    assert_eq!(
        items.items[0],
        ActionItem {
            title: "x".into(),
            owner: None
        }
    );
    let repair = &mock.requests()[1];
    assert_eq!(repair.messages.len(), 3);
    assert_eq!(repair.messages[1].text(), r#"{"items":[{"name":"x"}]}"#);
    let Part::Text(feedback) = &repair.messages[2].parts[0] else {
        panic!("the repair turn is text");
    };
    assert!(feedback.contains("missing field `title`"), "{feedback}");
    assert_eq!(harness.usage.calls().len(), 2);
}

#[tokio::test]
async fn a_failed_repair_is_invalid_output_without_the_content() {
    let mock = Arc::new(
        MockProvider::new()
            .reply(Route::Tagging, "not json at all, secret text")
            .reply(Route::Tagging, "still not json, secret text"),
    );
    let harness = mock_gateway(mock);

    let error = harness
        .gateway
        .structured::<ActionItems>(Route::Tagging, Prompt::new("x"))
        .await
        .unwrap_err();

    assert!(matches!(error, LlmError::InvalidOutput { .. }));
    assert!(!error.to_string().contains("secret"), "{error}");
    assert!(
        harness
            .usage
            .calls()
            .iter()
            .all(|call| call.outcome == CallOutcome::Ok)
    );
}
