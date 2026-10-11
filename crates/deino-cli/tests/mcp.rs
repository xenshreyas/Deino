//! `deino mcp` over the wire: the handshake, `tools/list`, a call to every
//! tool against the sample database, and structured errors.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail loudly, like the tests they serve"
)]

#[path = "support/mcp_client.rs"]
mod mcp_client;
#[path = "support/sample.rs"]
mod sample;

use mcp_client::{McpClient, PROTOCOL, text};
use sample::Sample;
use serde_json::{Value, json};

/// Every tool, in `tools/list` order.
const TOOLS: [&str; 10] = [
    "list_meetings",
    "read_meeting",
    "get_transcript",
    "search",
    "list_action_items",
    "list_people",
    "list_companies",
    "list_tags",
    "list_folders",
    "list_calendar_events",
];

/// A server over a fresh sample, past the handshake.
async fn connected() -> (Sample, McpClient) {
    let sample = Sample::new().await;
    let (client, _) = McpClient::connect(&sample.path);
    (sample, client)
}

/// Calls `tool`, checks it succeeded with text, and returns the result.
fn answer(client: &mut McpClient, tool: &str, arguments: &Value) -> Value {
    let result = client.call(tool, arguments);
    assert_ne!(result["isError"], true, "{tool}: {}", text(&result));
    assert!(!text(&result).is_empty(), "{tool} returned no text");
    result
}

/// The length of the array at `key` of a result's structured content.
fn count(result: &Value, key: &str) -> usize {
    result["structuredContent"][key].as_array().unwrap().len()
}

#[tokio::test]
async fn the_handshake_names_the_server_and_offers_tools() {
    let sample = Sample::new().await;

    let (_client, info) = McpClient::connect(&sample.path);

    let result = &info["result"];
    assert_eq!(result["protocolVersion"], PROTOCOL);
    assert_eq!(result["serverInfo"]["name"], "deino");
    assert!(result["capabilities"]["tools"].is_object());
    let instructions = result["instructions"].as_str().unwrap();
    assert!(instructions.contains("list_meetings"));
}

#[tokio::test]
async fn every_tool_is_listed_read_only_with_schemas() {
    let (_sample, mut client) = connected().await;

    let listed = client.request("tools/list", &json!({}));

    let tools = listed["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, TOOLS);
    for tool in tools {
        assert_eq!(
            tool["annotations"]["readOnlyHint"], true,
            "{}",
            tool["name"]
        );
        assert_eq!(tool["inputSchema"]["type"], "object");
        assert_eq!(tool["outputSchema"]["type"], "object");
        assert!(tool["inputSchema"]["properties"]["format"].is_object());
    }
    let catalogue: Vec<Value> = tools
        .iter()
        .map(|tool| {
            json!({ "name": tool["name"], "description": tool["description"], "input": tool["inputSchema"] })
        })
        .collect();
    insta::assert_json_snapshot!("tools_list", catalogue);
}

#[tokio::test]
async fn meeting_tools_list_read_and_page() {
    let (_sample, mut client) = connected().await;

    let meetings = answer(&mut client, "list_meetings", &json!({ "limit": 3 }));
    let read = answer(
        &mut client,
        "read_meeting",
        &json!({ "ids": ["meeting-fixture-standup"] }),
    );
    let page = answer(
        &mut client,
        "get_transcript",
        &json!({ "meeting_id": "meeting-fixture-standup", "limit": 5 }),
    );

    assert_eq!(count(&meetings, "meetings"), 3);
    assert!(text(&meetings).starts_with("# Meetings (3)"));
    assert!(text(&read).contains("## Action items"));
    assert_eq!(count(&page, "lines"), 5);
    assert!(page["structuredContent"]["nextFromMs"].is_number());
}

#[tokio::test]
async fn search_and_action_items_find_moments_and_commitments() {
    let (_sample, mut client) = connected().await;

    let found = answer(&mut client, "search", &json!({ "query": "release cut" }));
    let mine = answer(
        &mut client,
        "list_action_items",
        &json!({ "assignee": "me", "status": "any" }),
    );

    let hits = found["structuredContent"]["hits"].as_array().unwrap();
    assert!(hits.iter().any(|hit| hit["kind"] == "transcript"));
    let items = mine["structuredContent"]["items"].as_array().unwrap();
    assert!(!items.is_empty());
    assert!(items.iter().all(|item| item["assigneeIsMe"] == true));
}

#[tokio::test]
async fn directory_tools_list_people_companies_tags_folders_and_events() {
    let (_sample, mut client) = connected().await;

    let people = answer(
        &mut client,
        "list_people",
        &json!({ "query": "brightkite" }),
    );
    let companies = answer(&mut client, "list_companies", &json!({}));
    let tags = answer(&mut client, "list_tags", &json!({}));
    let folders = answer(&mut client, "list_folders", &json!({ "format": "json" }));
    let events = answer(
        &mut client,
        "list_calendar_events",
        &json!({ "from": "2026-10-08", "to": "2026-10-08" }),
    );

    assert_eq!(count(&people, "people"), 2);
    assert_eq!(count(&companies, "companies"), 5);
    assert_eq!(count(&tags, "tags"), 2);
    let as_json: Value = serde_json::from_str(text(&folders)).unwrap();
    assert_eq!(as_json, folders["structuredContent"]);
    assert_eq!(
        events["structuredContent"]["events"][0]["title"],
        "Eng standup"
    );
}

#[tokio::test]
async fn failures_are_tool_errors_the_model_can_read() {
    let (_sample, mut client) = connected().await;

    let missing = client.call("read_meeting", &json!({ "ids": ["meeting-nope"] }));
    let unknown = client.call("list_meetings", &json!({ "max_results": 3 }));
    let unknown_tool = client.request("tools/call", &json!({ "name": "delete_everything" }));

    let error = |result: &Value| -> Value { serde_json::from_str(text(result)).unwrap() };
    assert_eq!(missing["isError"], true);
    assert_eq!(error(&missing)["error"]["code"], "not_found");
    assert_eq!(error(&unknown)["error"]["code"], "invalid");
    let message = error(&unknown)["error"]["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(message.contains("max_results"), "{message}");
    assert!(unknown_tool["error"].is_object());
}

#[tokio::test]
async fn without_a_database_the_server_still_starts_and_explains() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deino.sqlite");
    let (mut client, info) = McpClient::connect(&path);

    let result = client.call("list_meetings", &json!({}));

    assert_eq!(info["result"]["serverInfo"]["name"], "deino");
    assert_eq!(result["isError"], true);
    let error: Value = serde_json::from_str(text(&result)).unwrap();
    assert_eq!(error["error"]["code"], "unavailable");
    assert!(!path.exists(), "the server never creates a database");
}
