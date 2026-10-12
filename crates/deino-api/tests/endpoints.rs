//! Search, the directory, tags and action items, over the seeded sample.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail loudly, like the tests they serve"
)]

mod support;

use std::sync::Arc;

use async_trait::async_trait;
use axum::http::{Method, StatusCode};
use deino_core::redaction::TextRedactor;
use serde_json::json;
use support::Harness;

/// Redacts every snippet whole, so a test can see redaction ran.
struct RedactAll;

#[async_trait]
impl TextRedactor for RedactAll {
    async fn redact(&self, texts: Vec<String>) -> Vec<String> {
        texts.iter().map(|_| "[redacted]".to_owned()).collect()
    }
}

#[tokio::test]
async fn search_finds_meetings_pages_and_refuses_a_blank_query() {
    let api = Harness::new().await;
    let token = api.writer().await;

    let hits = api.get("/v1/search?q=pricing&limit=1", &token).await;
    let blank = api.get("/v1/search?q=%20", &token).await;

    assert_eq!(hits.status, StatusCode::OK, "{}", hits.body);
    assert_eq!(hits.body["data"].as_array().unwrap().len(), 1);
    assert!(hits.body["data"][0]["snippet"].is_string());
    assert!(hits.body["nextCursor"].is_string());
    assert_eq!(blank.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn search_snippets_are_redacted_when_the_setting_is_on() {
    let api = Harness::with_redactor(Arc::new(RedactAll)).await;
    let token = api.writer().await;

    let hits = api.get("/v1/search?q=pricing", &token).await;

    let data = hits.body["data"].as_array().unwrap();
    assert!(!data.is_empty());
    assert!(data.iter().all(|hit| hit["snippet"] == "[redacted]"));
}

#[tokio::test]
async fn the_directory_lists_people_companies_folders_and_events() {
    let api = Harness::new().await;
    let token = api.writer().await;

    for uri in [
        "/v1/people?limit=2",
        "/v1/companies",
        "/v1/folders",
        "/v1/calendar/events?from=2026-09-28&to=2026-10-09",
    ] {
        let reply = api.get(uri, &token).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri}: {}", reply.body);
        assert!(!reply.body["data"].as_array().unwrap().is_empty(), "{uri}");
    }
    let shreyas = api.get("/v1/people?q=shreyas", &token).await;
    assert_eq!(shreyas.body["data"][0]["isMe"], true);
    let folders = api.get("/v1/folders", &token).await;
    assert!(
        folders.body["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["name"] == "Brightkite")
    );
}

#[tokio::test]
async fn tags_are_created_and_duplicates_conflict() {
    let api = Harness::new().await;
    let token = api.writer().await;

    let created = api
        .send(
            (Method::POST, "/v1/tags"),
            &token,
            json!({ "name": "Board" }),
        )
        .await;
    let duplicate = api
        .send(
            (Method::POST, "/v1/tags"),
            &token,
            json!({ "name": "board" }),
        )
        .await;

    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(created.body["name"], "Board");
    assert_eq!(duplicate.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn tags_are_renamed_and_deleted() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let created = api
        .send(
            (Method::POST, "/v1/tags"),
            &token,
            json!({ "name": "Board" }),
        )
        .await;
    let uri = format!("/v1/tags/{}", created.body["id"].as_str().unwrap());

    let renamed = api
        .send(
            (Method::PUT, &uri),
            &token,
            json!({ "name": "Board meetings" }),
        )
        .await;
    let deleted = api.delete(&uri, &token).await;
    let bad_id = api.delete("/v1/tags/not-an-id", &token).await;
    let listed = api.get("/v1/tags", &token).await;

    assert_eq!(renamed.body["name"], "Board meetings");
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert_eq!(bad_id.status, StatusCode::NOT_FOUND);
    let names: Vec<&str> = listed.body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tag| tag["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Customer", "Hiring"]);
}

const ALL_ITEMS: &str = "/v1/action-items?status=any&limit=100";

#[tokio::test]
async fn an_action_item_is_created_and_listed() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let before = api.get(ALL_ITEMS, &token).await.body["data"]
        .as_array()
        .unwrap()
        .len();
    let item = json!({ "title": "Send the deck", "due": "2026-10-12" });

    let created = api
        .send((Method::POST, "/v1/action-items"), &token, item)
        .await;
    let after = api.get(ALL_ITEMS, &token).await;

    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["due"], "2026-10-12");
    assert_eq!(after.body["data"].as_array().unwrap().len(), before + 1);
}

#[tokio::test]
async fn an_action_item_is_changed_and_deleted() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let item = json!({ "title": "Send the deck" });
    let created = api
        .send((Method::POST, "/v1/action-items"), &token, item)
        .await;
    let uri = format!("/v1/action-items/{}", created.body["id"].as_str().unwrap());

    let done = api
        .send((Method::PUT, &uri), &token, json!({ "status": "done" }))
        .await;
    let empty = api.send((Method::PUT, &uri), &token, json!({})).await;
    let deleted = api.delete(&uri, &token).await;
    let gone = api.delete(&uri, &token).await;

    assert_eq!(done.body["status"], "done");
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn action_items_filter_by_assignee_and_meeting() {
    let api = Harness::new().await;
    let token = api.writer().await;

    let mine = api
        .get("/v1/action-items?assignee=me&status=any&limit=100", &token)
        .await;
    let standup = api
        .get(
            "/v1/action-items?meetingId=meeting-fixture-standup&status=any",
            &token,
        )
        .await;

    assert_eq!(mine.status, StatusCode::OK);
    let mine = mine.body["data"].as_array().unwrap();
    assert!(!mine.is_empty());
    assert!(mine.iter().all(|item| item["assigneeIsMe"] == true));
    let standup = standup.body["data"].as_array().unwrap();
    assert!(
        standup
            .iter()
            .all(|item| item["meetingId"] == "meeting-fixture-standup")
    );
}
