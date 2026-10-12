//! `/v1/meetings`: lists, cursors, reads, transcripts, import, edits and
//! the trash, over the seeded sample.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail loudly, like the tests they serve"
)]

mod support;

use axum::http::{Method, StatusCode, header};
use deino_db::Table;
use serde_json::{Value, json};
use support::Harness;

const STANDUP: &str = "meeting-fixture-standup";

/// Every page from `uri` on, joined, and how many pages.
async fn all_pages(api: &Harness, token: &str, uri: &str) -> (Vec<Value>, usize) {
    let mut items = Vec::new();
    let mut next = uri.to_owned();
    let mut pages = 0;
    loop {
        let reply = api.get(&next, token).await;
        assert_eq!(reply.status, StatusCode::OK, "{next}: {}", reply.body);
        pages += 1;
        items.extend(reply.body["data"].as_array().unwrap().iter().cloned());
        let Some(cursor) = reply.body["nextCursor"].as_str() else {
            assert!(reply.headers.get(header::LINK).is_none());
            return (items, pages);
        };
        // Follow the RFC 8288 link, which carries the filters and the cursor.
        let link = reply.headers[header::LINK].to_str().unwrap();
        assert!(link.ends_with("; rel=\"next\""), "{link}");
        assert!(link.contains(&format!("cursor={cursor}")), "{link}");
        link[1..link.find('>').unwrap()].clone_into(&mut next);
    }
}

fn ids(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn pages_are_stable_and_join_into_the_whole_list() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let whole = api.get("/v1/meetings?limit=100", &token).await;
    let whole = ids(whole.body["data"].as_array().unwrap());

    let (paged, page_count) = all_pages(&api, &token, "/v1/meetings?limit=3").await;

    assert_eq!(ids(&paged), whole);
    assert!(page_count >= 3, "{page_count} pages");
}

#[tokio::test]
async fn a_meeting_added_between_pages_shifts_nothing() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let first = api.get("/v1/meetings?limit=3", &token).await;
    let cursor = first.body["nextCursor"].as_str().unwrap().to_owned();
    let import = json!({ "name": "Late arrival", "createdAt": "2026-10-09T15:00:00Z",
        "transcript": [{ "speaker": "A", "text": "Hi", "timestamp": 0 }] });
    let created = api
        .call((Method::POST, "/v1/meetings"), Some(&token), Some(import))
        .await;
    assert_eq!(created.status, StatusCode::CREATED);

    let (rest, _) = all_pages(
        &api,
        &token,
        &format!("/v1/meetings?limit=3&cursor={cursor}"),
    )
    .await;

    let seen: Vec<String> = ids(first.body["data"].as_array().unwrap());
    let rest = ids(&rest);
    assert!(
        rest.iter().all(|id| !seen.contains(id)),
        "a page repeated a meeting"
    );
    assert!(!rest.contains(&created.body["id"].as_str().unwrap().to_owned()));
}

#[tokio::test]
async fn filters_narrow_and_a_cursor_from_other_filters_is_refused() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let customers = api.get("/v1/meetings?tag=Customer", &token).await;
    assert_eq!(customers.body["data"].as_array().unwrap().len(), 2);

    let first = api.get("/v1/meetings?limit=2", &token).await;
    let cursor = first.body["nextCursor"].as_str().unwrap();
    let mixed = api
        .get(
            &format!("/v1/meetings?limit=2&tag=Hiring&cursor={cursor}"),
            &token,
        )
        .await;
    let unknown = api.get("/v1/meetings?colour=red", &token).await;

    assert_eq!(mixed.status, StatusCode::BAD_REQUEST);
    assert_eq!(mixed.body["error"]["code"], "invalid");
    assert_eq!(unknown.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_meeting_reads_in_full_with_its_transcript_on_request() {
    let api = Harness::new().await;
    let token = api.writer().await;

    let plain = api.get(&format!("/v1/meetings/{STANDUP}"), &token).await;
    let full = api
        .get(
            &format!("/v1/meetings/{STANDUP}?include=transcript"),
            &token,
        )
        .await;
    let missing = api.get("/v1/meetings/meeting-nope", &token).await;
    let odd = api
        .get(&format!("/v1/meetings/{STANDUP}?include=audio"), &token)
        .await;

    assert_eq!(plain.status, StatusCode::OK);
    assert_eq!(plain.body["id"], STANDUP);
    assert!(plain.body["aiNotes"].is_string());
    assert!(plain.body.get("transcript").is_none());
    let lines = full.body["transcript"].as_array().unwrap();
    assert_eq!(
        lines.len() as u64,
        full.body["transcriptSegments"].as_u64().unwrap()
    );
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(odd.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_transcript_pages_by_cursor() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let whole = api
        .get(
            &format!("/v1/meetings/{STANDUP}?include=transcript"),
            &token,
        )
        .await;

    let uri = format!("/v1/meetings/{STANDUP}/transcript?limit=4");
    let (paged, page_count) = all_pages(&api, &token, &uri).await;

    assert_eq!(&paged, whole.body["transcript"].as_array().unwrap());
    assert!(page_count > 1);
}

#[tokio::test]
async fn an_imported_transcript_becomes_a_meeting() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let body = json!({
        "name": "Vendor call",
        "tags": [{ "name": "Customer" }],
        "notes": "## Decisions\n\n- Ship Friday",
        "transcript": [
            { "speaker": "Ana", "text": "Shall we ship Friday?", "timestamp": 0.0 },
            { "speaker": "Me", "words": [
                { "text": "Yes", "startTimestamp": 3.0, "endTimestamp": 3.4 }
            ] }
        ]
    });

    let created = api
        .call((Method::POST, "/v1/meetings"), Some(&token), Some(body))
        .await;
    let bad = api
        .call(
            (Method::POST, "/v1/meetings"),
            Some(&token),
            Some(json!({ "name": "x" })),
        )
        .await;

    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["title"], "Vendor call");
    assert_eq!(created.body["tags"], json!(["Customer"]));
    assert_eq!(created.body["transcriptSegments"], 2);
    assert!(
        created.body["aiNotes"]
            .as_str()
            .unwrap()
            .contains("Ship Friday")
    );
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_edit_replaces_title_notes_and_tags_and_tells_the_ui() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let tags = api.get("/v1/tags", &token).await;
    let hiring = tags.body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tag| tag["name"] == "Hiring")
        .unwrap()["id"]
        .clone();
    let mut notices = api.database.changes().subscribe();
    let edit = json!({ "title": "Daily standup", "myNotes": "- ask about CI", "tagIds": [hiring] });

    let reply = api
        .call(
            (Method::PUT, &format!("/v1/meetings/{STANDUP}")),
            Some(&token),
            Some(edit),
        )
        .await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["title"], "Daily standup");
    assert!(
        reply.body["myNotes"]
            .as_str()
            .unwrap()
            .contains("ask about CI")
    );
    assert_eq!(reply.body["tags"], json!(["Hiring"]));
    let changed = notices.recv().await.unwrap();
    assert!(changed.tables.contains(&Table::Meetings), "{changed:?}");
}

#[tokio::test]
async fn delete_moves_a_meeting_to_the_trash() {
    let api = Harness::new().await;
    let token = api.writer().await;
    let uri = format!("/v1/meetings/{STANDUP}");

    let deleted = api.call((Method::DELETE, &uri), Some(&token), None).await;
    let again = api.call((Method::DELETE, &uri), Some(&token), None).await;
    let list = api.get("/v1/meetings?limit=100", &token).await;

    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert_eq!(again.status, StatusCode::NOT_FOUND);
    assert!(!ids(list.body["data"].as_array().unwrap()).contains(&STANDUP.to_owned()));
    let trashed: Option<String> =
        sqlx::query_scalar("SELECT deleted_at FROM meetings WHERE id = ?")
            .bind(STANDUP)
            .fetch_one(api.database.reader())
            .await
            .unwrap();
    assert!(trashed.is_some(), "the row stays, in the trash");
}
